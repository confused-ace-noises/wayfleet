use std::{borrow::Borrow, ops::Deref, str::FromStr};
use derive_more::Debug;
use knus::{Decode, DecodeScalar, errors::{DecodeError, ExpectedType}, traits::ErrorSpan};
use miette::miette;
use bitflags::bitflags;
use smithay::input::keyboard::ModifiersState;
use xkbcommon::xkb::{KEYSYM_CASE_INSENSITIVE, keysym_from_name, keysyms::KEY_NoSymbol};
use xkeysym::Keysym;

bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct Modifiers : u8 {
        const DEFAULT          = 1;
        const SUPER            = 1 << 1;
        const CTRL             = 1 << 2;
        const SHIFT            = 1 << 3;
        const ALT              = 1 << 4;
        const ISO_LEVEL3_SHIFT = 1 << 5;
        const ISO_LEVEL5_SHIFT = 1 << 6;
    }
}

impl<S: ErrorSpan> DecodeScalar<S> for Modifiers {
    fn type_check(type_name: &Option<knus::span::Spanned<knus::ast::TypeName, S>>, ctx: &mut knus::decode::Context<S>) {
        if let Some(typ) = type_name {
            ctx.emit_error(DecodeError::TypeName {
                span: typ.span().clone(),
                found: Some(typ.deref().clone()),
                expected: ExpectedType::no_type(),
                rust_type: "String",
            });
        }
    }

    fn raw_decode(
        value: &knus::span::Spanned<knus::ast::Literal, S>,
        ctx: &mut knus::decode::Context<S>,
    ) -> Result<Self, DecodeError<S>> {
        let x = match &**value {
            knus::ast::Literal::String(string) => Self::from_str(string.as_ref()),
            knus::ast::Literal::Null => Ok(Self::empty()),
            _ => todo!(),
        }.map_err(|_| DecodeError::unexpected(value, "String", "unexpected modifier name"));


        match x {
            Ok(modifier) => {
                if (modifier & Modifiers::DEFAULT).bits() > 0 {
                    ctx.emit_error(DecodeError::unsupported(value, "default modifier is unsupported at this location"));
                    Ok(Modifiers::SUPER)
                } else {
                    Ok(modifier)
                }
            }

            Err(err) => {
                ctx.emit_error(err);
                Ok(Modifiers::SUPER)
            },
        }
    }
}

impl FromStr for Modifiers {
    type Err = miette::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_ascii_lowercase().as_str() {
            "mod" | "default"        => Ok(Modifiers::DEFAULT),
            "super" | "win"          => Ok(Modifiers::SUPER),
            "ctrl" | "control"       => Ok(Modifiers::CTRL),
            "shift"                  => Ok(Modifiers::SHIFT),
            "alt"                    => Ok(Modifiers::ALT),
            "isolevel3shift" 
                | "iso_level3_shift" 
                | "mod3" 
                | "altgr"            => Ok(Modifiers::ISO_LEVEL3_SHIFT),
            
            "isolevel5shift" 
                | "iso_level5_shift" 
                | "mod5"             => Ok(Modifiers::ISO_LEVEL5_SHIFT),
            
            s => Err(miette!("invalid modifier: {s}"))
        } 
    }
}

impl<B: Borrow<ModifiersState>> From<B> for Modifiers {
    fn from(value: B) -> Self {
        let ModifiersState { ctrl, alt, shift, logo, iso_level3_shift, iso_level5_shift, ..}: &ModifiersState = value.borrow();
        
        let mut modifiers = Modifiers::empty();
        
        if *ctrl {
            modifiers |= Modifiers::CTRL;
        }

        if *alt {
            modifiers |= Modifiers::ALT
        }

        if *shift {
            modifiers |= Modifiers::SHIFT
        }

        if *logo {
            modifiers |= Modifiers::SUPER
        }

        if *iso_level3_shift {
            modifiers |= Modifiers::ISO_LEVEL3_SHIFT
        }

        if *iso_level5_shift {
            modifiers |= Modifiers::ISO_LEVEL5_SHIFT
        }

        modifiers
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum Trigger {
    Keysym(Keysym),
    // TODO: add stuff
}

impl FromStr for Trigger {
    type Err = miette::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        #[allow(clippy::match_single_binding)]
        match s {
            key => {
                let keysym = keysym_from_name(key, KEYSYM_CASE_INSENSITIVE);
                println!("{keysym:?}");
                if keysym.raw() == KEY_NoSymbol {
                    return Err(miette!("invalid key: {key}"));
                }
                Ok(Trigger::Keysym(keysym))
            } 
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct KeyCombo {
    pub modifiers: Modifiers,
    pub trigger: Trigger
}

impl KeyCombo {
    pub fn is_it(&self, other: &Self, default: Modifiers) -> bool {
        let KeyCombo { modifiers, trigger } = self;
        
        let mut tmp_mods = *modifiers;
        let mut tmp_other = other.modifiers;
        
        if tmp_other.contains(Modifiers::DEFAULT) || tmp_other.contains(default) {
            tmp_other.remove(Modifiers::DEFAULT);
            tmp_other |= default;
        }

        if tmp_mods.contains(Modifiers::DEFAULT) || tmp_mods.contains(default) {
            tmp_mods.remove(Modifiers::DEFAULT);
            tmp_mods |= default;
        }
        
        tmp_mods == tmp_other && *trigger == other.trigger
    } 
}

impl FromStr for KeyCombo {
    type Err = miette::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut split = s.split('+');
        let key = split.next_back().unwrap();

        let mut modifiers = Modifiers::empty();

        for part in split{
            let part = part.trim();

            modifiers |= part.parse::<Modifiers>()?;
        }

        let trigger = key.trim().parse::<Trigger>()?;

        Ok(Self {
            modifiers,
            trigger
        })
    }
}

#[derive(Debug, Decode, Clone)]
pub enum Action {
    // * move focus
    MoveFocusUp,
    MoveFocusDown,
    MoveFocusRight,
    MoveFocusLeft,

    // * move (map) or swap (map & privileged) window
    MoveOrSwapUp,
    MoveOrSwapDown,
    MoveOrSwapRight,
    MoveOrSwapLeft,

    // * push laterally in privileged
    PushLateralRight,
    PushLateralLeft,

    // * spawn
    Spawn(#[knus(arguments)] Vec<String>),
    SpawnSh(#[knus(argument)] String),

    // * misc
    CloseWindow,
    Quit,

    // * Diag
    DumpMap,
    DumpPrivileged,
    DumpLayout, 

    #[knus(skip)]
    None,
}

#[derive(Debug, Decode)]
pub struct KeyBinds {
    #[knus(child, unwrap(argument))]
    pub mod_key: Modifiers,

    #[knus(children)]
    pub keybinds: Vec<KeyBind>,
}

#[derive(Debug, Clone)]
pub struct KeyBind {
    pub combo: KeyCombo,
    pub action: Action,
}

impl<S: ErrorSpan> Decode<S> for KeyBind {
    fn decode_node(node: &knus::ast::SpannedNode<S>, ctx: &mut knus::decode::Context<S>) -> Result<Self, knus::errors::DecodeError<S>> {
        if let Some(type_name) = &node.type_name {
            ctx.emit_error(DecodeError::unexpected(
                type_name,
                "type name",
                "no type name expected for this node",
            ));
        }

        for arg in node.arguments.iter() {
            ctx.emit_error(DecodeError::unexpected(
                &arg.literal,
                "argument",
                "no arguments expected for this node",
            ));
        }

        for prop in node.properties.iter() {
            ctx.emit_error(DecodeError::unexpected(
                prop.0,
                "property",
                "no properties expected for this node",
            ));
        }

        let key_combo = node
            .node_name
            .parse::<KeyCombo>()
            .map_err(|e| DecodeError::conversion(&node.node_name, e.wrap_err("invalid keybind")))?;

        let mut children = node.children();

        let mut action = Action::None;

        if let Some(child) = children.next() {
            action = Action::decode_node(child, ctx)?;
        } else {
            ctx.emit_error(DecodeError::missing(node, "expected an action for this keybind"));
        }

        Ok(Self {
            action,
            combo: key_combo
        })
    }
}