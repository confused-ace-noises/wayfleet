//! This code (and everything in this module) is a nightmare. If you ever find yourself
//! in the state to want to fix this thing, refrain from doing so if you don't wanna go 
//! insane. In the words of an italian guy a lot more word-inclined than me,
//! 
//! Per me si va ne la città dolente,
//! per me si va ne l'etterno dolore,
//! per me si va tra la perduta gente.
//! [...]
//! Lasciate ogne speranza, voi ch'intrate.

use std::{
    borrow::Borrow, marker::PhantomData, sync::{Arc, RwLock},
};

use smithay::{
    backend::renderer::{
        Color32F, Renderer, RendererSuper, element::{
            AsRenderElements, Element, Id, Kind, RenderElement,
            surface::{WaylandSurfaceRenderElement, render_elements_from_surface_tree},
            utils::CropRenderElement,
        }, gles::{GlesRenderer, Uniform, element::PixelShaderElement}, utils::CommitCounter,
    }, desktop::{PopupManager, Window, WindowSurface, space::SpaceElement}, render_elements, utils::{Buffer, IsAlive, Logical, Physical, Point, Rectangle, Scale},
};

use crate::{state::CONFIG, winit::BORDER_SHADER};

pub mod controller;
pub mod map;
pub mod privileged;

pub type CropRect = Arc<RwLock<Rectangle<i32, Logical>>>;

#[derive(Debug, Clone)]
pub enum WayfleetWindowType {
    Map(CropRect),
    Privileged(CropRect),
}

impl PartialEq for WayfleetWindowType {
    fn eq(&self, other: &Self) -> bool {
        core::mem::discriminant(self) == core::mem::discriminant(other)
    }
}

#[derive(Debug, Clone, derive_more::Deref, derive_more::DerefMut)]
pub struct WayfleetWindow {
    #[deref]
    #[deref_mut]
    pub window: Window,
    pub window_type: WayfleetWindowType,
    pub specific_crop: CropRect,
    pub is_focused: Arc<RwLock<bool>>,
}

impl WayfleetWindow {
    pub fn new(window: Window, window_type: WayfleetWindowType, specific_crop: CropRect, is_focused: bool) -> Self {
        Self {
            window,
            window_type,
            specific_crop,
            is_focused: Arc::new(RwLock::new(is_focused))
        }
    }

    pub fn is_map(&self) -> bool {
        matches!(self.window_type, WayfleetWindowType::Map(_))
    }

    pub fn focused(&mut self, focus: bool) {
        let mut lock = self.is_focused.write().unwrap();
        *lock = focus;
    }

    pub fn dummy(window: Window) -> Self {
        Self::new(window, WayfleetWindowType::Privileged(Arc::new(RwLock::new(Rectangle::zero()))), Arc::new(RwLock::new(Rectangle::zero())), false)
    }

    #[allow(non_snake_case)]
    pub fn new_x11_OR(window: Window) -> Self {
        // note: this is allowed solely because the renderer checks wherther a window
        // is OR before cropping it and stuff. this will hopefully not shoot me in the
        // foot in the future (it definitely will)
        Self::dummy(window)
    }
}

impl PartialEq for WayfleetWindow {
    fn eq(&self, other: &Self) -> bool {
        self.window == other.window
    }
}

impl<T: Borrow<Window>> PartialEq<T> for WayfleetWindow {
    fn eq(&self, other: &T) -> bool {
        let window = other.borrow();

        self.window == *window
    }
}

impl Eq for WayfleetWindow {}

impl std::hash::Hash for WayfleetWindow {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.window.hash(state);
    }
}

// delegate to window, maybe use delegate! macro?
impl SpaceElement for WayfleetWindow {
    fn bbox(&self) -> smithay::utils::Rectangle<i32, smithay::utils::Logical> {
        self.window.bbox()
    }

    fn is_in_input_region(
        &self,
        point: &smithay::utils::Point<f64, smithay::utils::Logical>,
    ) -> bool {
        let rw_lock = match &self.window_type {
            WayfleetWindowType::Map(rw_lock) => rw_lock,
            WayfleetWindowType::Privileged(rw_lock) => rw_lock,
        };
        
        let mut lock = *rw_lock.read().unwrap();
        
        // TODO: put this on the assign op itself
        lock.loc = Point::new(0, 0);
        if (lock).to_f64().contains(*point) {
            self.window.is_in_input_region(point)
        } else {
            false
        }
    }

    fn set_activate(&self, activated: bool) {
        self.window.set_activate(activated);
    }

    fn output_enter(
        &self,
        output: &smithay::output::Output,
        overlap: smithay::utils::Rectangle<i32, smithay::utils::Logical>,
    ) {
        self.window.output_enter(output, overlap);
    }

    fn output_leave(&self, output: &smithay::output::Output) {
        self.window.output_leave(output);
    }
}

impl IsAlive for WayfleetWindow {
    fn alive(&self) -> bool {
        self.window.alive()
    }
}

pub trait IntoWayfleetWindow {
    fn as_map(&self, viewport: CropRect, crop_rect: CropRect) -> WayfleetWindow;
    fn as_priv(&self, viewport: CropRect, crop_rect: CropRect) -> WayfleetWindow;
}

impl IntoWayfleetWindow for Window {
    fn as_map(&self, viewoport: CropRect, crop_rect: CropRect) -> WayfleetWindow {
        WayfleetWindow {
            window: self.clone(),
            window_type: WayfleetWindowType::Map(viewoport),
            specific_crop: crop_rect,
            is_focused: Arc::new(RwLock::new(false)),
        }
    }

    fn as_priv(&self, viewoport: CropRect, crop_rect: CropRect) -> WayfleetWindow {
        WayfleetWindow {
            window: self.clone(),
            window_type: WayfleetWindowType::Privileged(viewoport),
            specific_crop: crop_rect,
            is_focused: Arc::new(RwLock::new(false)),
        }
    }
}

impl AsRenderElements<GlesRenderer> for WayfleetWindow {
    type RenderElement = InnerOrBorder<GlesRenderer, MaybeCropped<GlesRenderer, WaylandSurfaceRenderElement<GlesRenderer>>>;

    fn render_elements<C: From<Self::RenderElement>>(
        &self,
        renderer: &mut GlesRenderer,
        location: smithay::utils::Point<i32, smithay::utils::Physical>,
        scale: smithay::utils::Scale<f64>,
        alpha: f32,
    ) -> Vec<C> {
        let wayfleet_config::Config { layout: wayfleet_config::Layout { decorations: wayfleet_config::decorations::Decorations { border }, .. }, .. } = CONFIG.get().unwrap().as_ref();

        let render_with_borders = |x: WaylandSurfaceRenderElement<GlesRenderer>| {
            
            let specific_crop: Rectangle<i32, Logical> = *self.specific_crop.read().unwrap();

            let color = if *self.is_focused.read().unwrap() {
                border.active_color.clone()
            } else {
                border.inactive_color.clone()
            };

            let viewport_crop = if self.is_map() {
                let Self { window_type: WayfleetWindowType::Map(crop), .. } = self else { unreachable!() };
                *crop.read().unwrap()
            } else {
                let Self { window_type: WayfleetWindowType::Privileged(crop), .. } = self else { unreachable!() };
                *crop.read().unwrap()
            };

            let final_rect = viewport_crop.intersection(specific_crop).unwrap_or_default();

            CropRenderElement::from_element(x, scale, final_rect.to_physical_precise_round(scale))
            .map(|crop| {
                [
                    InnerOrBorder::Inner(MaybeCropped::Crop(crop)),
                    InnerOrBorder::Border(make_border(final_rect, border.width, alpha, border.corner_radius as f32, color.into()))
                ]
            }).into_iter().flatten().map(C::from)
        };

        match self.underlying_surface() {
            WindowSurface::Wayland(s) => {
                let mut render_elements: Vec<C> = Vec::new();
                let surface = s.wl_surface();
                let popup_render_elements =
                    PopupManager::popups_for_surface(surface).flat_map(|(popup, popup_offset)| {
                        let geo = popup.geometry();
                        let offset = (self.geometry().loc + popup_offset - popup.geometry().loc)
                            .to_physical_precise_round(scale);

                        render_elements_from_surface_tree::<
                            _,
                            WaylandSurfaceRenderElement<GlesRenderer>,
                        >(
                            renderer,
                            popup.wl_surface(),
                            location + offset,
                            scale,
                            alpha,
                            Kind::Unspecified,
                        )
                        .into_iter()
                        .flat_map(move |x| {
                            let el = MaybeCropped::NoCrop(x);

                            let x = make_border(geo, border.width, alpha, border.corner_radius as f32, if *self.is_focused.read().unwrap() { border.active_color.clone() } else { border.inactive_color.clone() }.into());

                            let ready = [InnerOrBorder::Inner(el), InnerOrBorder::Border(x)];
                            ready.into_iter().map(C::from)
                        })
                    });

                render_elements.extend(popup_render_elements);

                render_elements.extend(
                    render_elements_from_surface_tree(
                        renderer,
                        surface,
                        location,
                        scale,
                        alpha,
                        Kind::Unspecified,
                    )
                    .into_iter()
                    .flat_map(
                        |x: WaylandSurfaceRenderElement<GlesRenderer>| {
                            render_with_borders(x)
                        },
                    ),
                );

                render_elements
            },

            WindowSurface::X11(x11) => {
                let mut render_elements = Vec::new();

                if !x11.is_override_redirect() {
                    let x = render_elements_from_surface_tree(renderer, &x11.wl_surface().unwrap(), location, scale, alpha, Kind::Unspecified)
                        .into_iter()
                        .flat_map(render_with_borders);

                    render_elements.extend(x);
                } else {
                    let x = render_elements_from_surface_tree(renderer, &x11.wl_surface().unwrap(), location, scale, alpha, Kind::Unspecified)
                        .into_iter()
                        .map(|x: WaylandSurfaceRenderElement<GlesRenderer>| {
                            let element = MaybeCropped::NoCrop(x);

                            C::from(InnerOrBorder::Inner(element))
                        });

                    render_elements.extend(x);
                }

                render_elements
            },
        }
    }
}

render_elements! {
    pub MaybeCropped<R, X>
    where
        R: Renderer;
        // X: RenderElement<R> + Element;
    Crop=CropRenderElement<X>,
    NoCrop=X,
}

fn make_border(mut window_rect: Rectangle<i32, Logical>, thicc: u32, alpha: f32, radius: f32, color: Color32F) -> PixelShaderElement {
    window_rect.loc -= Point::new(thicc as i32, thicc as i32);
    window_rect.size += (thicc as i32 * 2, thicc as i32 * 2).into();

    // ! DEBUG!
    // window_rect.loc += Point::new(500, 0);

    let x = color.components();

    PixelShaderElement::new(
        BORDER_SHADER.get().unwrap().clone(),
        window_rect,
        None,
        alpha,
        vec![
            Uniform::new("border_color", x),
            Uniform::new("border_thickness", thicc as f32),
            Uniform::new("corner_radius", radius)
        ],
        Kind::default(),
    )
}

pub enum InnerOrBorder<R: Renderer, E: RenderElement<R>> {
    Inner(E),
    Border(PixelShaderElement),
    IdekAnymore(PhantomData<R>),
}

impl<R: Renderer, E: RenderElement<R>> Element for InnerOrBorder<R, E> {
    fn id(&self) -> &Id {
        match self {
            InnerOrBorder::Inner(e) => e.id(),
            InnerOrBorder::Border(border) => border.id(),
            InnerOrBorder::IdekAnymore(_) => unimplemented!(),
        }
    }

    fn current_commit(&self) -> CommitCounter {
        match self {
            InnerOrBorder::Inner(e) => e.current_commit(),
            InnerOrBorder::Border(border) => border.current_commit(),
            InnerOrBorder::IdekAnymore(_) => unimplemented!(),
        }
    }

    fn src(&self) -> Rectangle<f64, Buffer> {
        match self {
            InnerOrBorder::Inner(e) => e.src(),
            InnerOrBorder::Border(border) => border.src(),
            InnerOrBorder::IdekAnymore(_) => unimplemented!(),
        }
    }

    fn geometry(&self, scale: Scale<f64>) -> Rectangle<i32, Physical> {
        match self {
            InnerOrBorder::Inner(e) => e.geometry(scale),
            InnerOrBorder::Border(border) => border.geometry(scale),
            InnerOrBorder::IdekAnymore(_) => unimplemented!(),
        }
    }

    fn location(&self, scale: Scale<f64>) -> Point<i32, Physical> {
        match self {
            InnerOrBorder::Inner(e) => e.location(scale),
            InnerOrBorder::Border(border) => border.location(scale),
            InnerOrBorder::IdekAnymore(_) => unimplemented!(),
        }
    }

    fn transform(&self) -> smithay::utils::Transform {
        match self {
            InnerOrBorder::Inner(e) => e.transform(),
            InnerOrBorder::Border(border) => border.transform(),
            InnerOrBorder::IdekAnymore(_) => unimplemented!(),
        }
    }

    fn damage_since(
        &self,
        scale: Scale<f64>,
        commit: Option<CommitCounter>,
    ) -> smithay::backend::renderer::utils::DamageSet<i32, Physical> {
        match self {
            InnerOrBorder::Inner(e) => e.damage_since(scale, commit),
            InnerOrBorder::Border(border) => border.damage_since(scale, commit),
            InnerOrBorder::IdekAnymore(_) => unimplemented!(),
        }
    }

    fn opaque_regions(
        &self,
        scale: Scale<f64>,
    ) -> smithay::backend::renderer::utils::OpaqueRegions<i32, Physical> {
        match self {
            InnerOrBorder::Inner(e) => e.opaque_regions(scale),
            InnerOrBorder::Border(border) => border.opaque_regions(scale),
            InnerOrBorder::IdekAnymore(_) => unimplemented!(),
        }
    }

    fn alpha(&self) -> f32 {
        match self {
            InnerOrBorder::Inner(e) => e.alpha(),
            InnerOrBorder::Border(border) => border.alpha(),
            InnerOrBorder::IdekAnymore(_) => unimplemented!(),
        }
    }

    fn kind(&self) -> Kind {
        match self {
            InnerOrBorder::Inner(e) => e.kind(),
            InnerOrBorder::Border(border) => border.kind(),
            InnerOrBorder::IdekAnymore(_) => unimplemented!(),
        }
    }

    fn is_framebuffer_effect(&self) -> bool {
        match self {
            InnerOrBorder::Inner(e) => e.is_framebuffer_effect(),
            InnerOrBorder::Border(border) => border.is_framebuffer_effect(),
            InnerOrBorder::IdekAnymore(_) => unimplemented!(),
        }
    }
}

impl< E: RenderElement<GlesRenderer>> RenderElement<GlesRenderer> for InnerOrBorder<GlesRenderer, E> {
    fn draw(
        &self,
        frame: &mut <GlesRenderer as RendererSuper>::Frame<'_, '_>,
        src: Rectangle<f64, Buffer>,
        dst: Rectangle<i32, Physical>,
        damage: &[Rectangle<i32, Physical>],
        opaque_regions: &[Rectangle<i32, Physical>],
        cache: Option<&smithay::utils::user_data::UserDataMap>,
    ) -> Result<(), <GlesRenderer as RendererSuper>::Error> {
        match self {
            InnerOrBorder::Inner(e) => e.draw(frame, src, dst, damage, opaque_regions, cache),
            InnerOrBorder::Border(border) => {
                RenderElement::<GlesRenderer>::draw(border, frame, src, dst, damage, opaque_regions, cache)
            }
            InnerOrBorder::IdekAnymore(_) => unimplemented!(),
        }
    }
}
