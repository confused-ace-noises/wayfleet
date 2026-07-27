use std::{borrow::Borrow, sync::{Arc, RwLock}};

use smithay::{backend::renderer::{Renderer, element::{AsRenderElements, Kind, surface::{WaylandSurfaceRenderElement, render_elements_from_surface_tree}, utils::CropRenderElement}, gles::GlesRenderer}, desktop::{PopupManager, Window, WindowSurface, space::SpaceElement}, render_elements, utils::{IsAlive, Logical, Rectangle}};

pub mod controller;
pub mod map;
pub mod privileged;



#[derive(Debug, Clone)]
pub enum WayfleetWindowType {
    Map(Arc<RwLock<Rectangle<i32, Logical>>>),
    Privileged,
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
    pub window_type: WayfleetWindowType
}

impl WayfleetWindow {
    pub fn new(window: Window, window_type: WayfleetWindowType) -> Self {
        Self { window, window_type }
    }

    pub fn is_map(&self) -> bool {
        matches!(self.window_type, WayfleetWindowType::Map(_))
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

    fn is_in_input_region(&self, point: &smithay::utils::Point<f64, smithay::utils::Logical>) -> bool {
        self.window.is_in_input_region(point)
    }

    fn set_activate(&self, activated: bool) {
        self.window.set_activate(activated);
    }

    fn output_enter(&self, output: &smithay::output::Output, overlap: smithay::utils::Rectangle<i32, smithay::utils::Logical>) {
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
    fn as_map(&self, rect: Arc<RwLock<Rectangle<i32, Logical>>>) -> WayfleetWindow;
    fn as_priv(&self) -> WayfleetWindow;
}

impl IntoWayfleetWindow for Window {
    fn as_map(&self, rect: Arc<RwLock<Rectangle<i32, Logical>>>) -> WayfleetWindow {
        WayfleetWindow { window: self.clone(), window_type: WayfleetWindowType::Map(rect) }
    }

    fn as_priv(&self) -> WayfleetWindow {
        WayfleetWindow { window: self.clone(), window_type: WayfleetWindowType::Privileged }
    }
}

impl AsRenderElements<GlesRenderer> for WayfleetWindow {
    type RenderElement = MaybeCropped<GlesRenderer, WaylandSurfaceRenderElement<GlesRenderer>>;

    fn render_elements<C: From<Self::RenderElement>>(
        &self,
        renderer: &mut GlesRenderer,
        location: smithay::utils::Point<i32, smithay::utils::Physical>,
        scale: smithay::utils::Scale<f64>,
        alpha: f32,
    ) -> Vec<C> {
        match self.underlying_surface() {
            WindowSurface::Wayland(s) => {
                let mut render_elements: Vec<C> = Vec::new();
                let surface = s.wl_surface();
                let popup_render_elements =
                    PopupManager::popups_for_surface(surface).flat_map(|(popup, popup_offset)| {
                        let offset = (self.geometry().loc + popup_offset - popup.geometry().loc)
                            .to_physical_precise_round(scale);

                        render_elements_from_surface_tree::<_, WaylandSurfaceRenderElement<GlesRenderer>>(
                            renderer,
                            popup.wl_surface(),
                            location + offset,
                            scale,
                            alpha,
                            Kind::Unspecified,
                        ).into_iter()
                        .map(|x| C::from(Self::RenderElement::from(x)))
                    });

                render_elements.extend(popup_render_elements);

                render_elements.extend(render_elements_from_surface_tree(
                    renderer,
                    surface,
                    location,
                    scale,
                    alpha,
                    Kind::Unspecified,
                ).into_iter().filter_map(|x: WaylandSurfaceRenderElement<GlesRenderer>| {
                    Some(C::from(if self.is_map() {
                        let Self { window_type: WayfleetWindowType::Map(clip), .. } = self else { unreachable!() }; 
                        let lock = clip.read().unwrap();
                        Self::RenderElement::from(CropRenderElement::from_element(x, scale, lock.to_physical_precise_round(scale))?)
                        // Self::RenderElement::from(x)
                    } else {
                        Self::RenderElement::from(x)
                    }))
                }));
                
                render_elements
            }
            // WindowSurface::X11(s) => AsRenderElements::render_elements(s, renderer, location, scale, alpha),
            WindowSurface::X11(_s) => todo!()
            
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

// pub enum MaybeCropped<E: Element> {
//     Crop(CropRenderElement<E>),
//     NoCrop(E)
// }

// impl<R: Renderer, E: Element> RenderElement<R> for MaybeCropped<E> {
//     fn draw(
//         &self,
//         frame: &mut <R>::Frame<'_, '_>,
//         src: smithay::utils::Rectangle<f64, smithay::utils::Buffer>,
//         dst: smithay::utils::Rectangle<i32, smithay::utils::Physical>,
//         damage: &[smithay::utils::Rectangle<i32, smithay::utils::Physical>],
//         opaque_regions: &[smithay::utils::Rectangle<i32, smithay::utils::Physical>],
//         cache: Option<&smithay::utils::user_data::UserDataMap>,
//     ) -> Result<(), <R>::Error> {
//         todo!()
//     }
// }
