use libdof::prelude::{Dof, Finger, Key, NamedFingering, Pos, SpecialKey};

pub struct Layout {
    dof: Dof,
}

impl Layout {
    pub fn load<P: AsRef<std::path::Path>>(path: P) -> Result<Self, String> {
        let s = std::fs::read_to_string(path).map_err(|e| e.to_string())?;

        serde_json::from_str::<Dof>(&s)
            .map(|dof| Self { dof })
            .map_err(|e| e.to_string())
    }

    pub fn finger_seq<const N: usize>(&self, keys: [Key; N]) -> [Option<(Finger, Pos)>; N] {
        keys.map(|k| {
            let key = match k {
                c @ Key::Char(_) => c,
                Key::Special(s) => match s {
                    sp @ SpecialKey::Repeat => Key::Special(sp),
                    sp @ SpecialKey::Space => Key::Special(sp),
                    sp @ SpecialKey::Tab => Key::Special(sp),
                    sp @ SpecialKey::Enter => Key::Special(sp),
                    _ => return None,
                },
                _ => return None,
            };

            let keyposes = self.dof.get(key);
            let keypos = keyposes.first()?;
            let finger = self.dof.finger(keypos.pos)?;

            Some((finger, keypos.pos))
        })
    }

    pub fn info(&self) -> LayoutInfo {
        LayoutInfo {
            name: self.dof.name().to_owned(),
            fingermap: self.dof.fingering_name().cloned(),
        }
    }
}

#[derive(Debug)]
pub struct LayoutInfo {
    name: String,
    fingermap: Option<NamedFingering>,
}

impl std::fmt::Display for LayoutInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut builder = tabled::builder::Builder::new();

        builder.push_record(["Layout name", &self.name]);
        if let Some(fingermap) = &self.fingermap {
            builder.push_record(["Fingermap", &fingermap.to_string()]);
        }

        let mut table = builder.build();

        table.with(tabled::settings::Style::modern_rounded());

        write!(f, "{}", table)
    }
}

pub fn parse_key(s: &str) -> Result<Key, String> {
    let key = match s {
        "Escape" => Key::Special(SpecialKey::Esc),
        "Backquote" => Key::Char('`'),
        "Digit1" => Key::Char('1'),
        "Digit2" => Key::Char('2'),
        "Digit3" => Key::Char('3'),
        "Digit4" => Key::Char('4'),
        "Digit5" => Key::Char('5'),
        "Digit6" => Key::Char('6'),
        "Digit7" => Key::Char('7'),
        "Digit8" => Key::Char('8'),
        "Digit9" => Key::Char('9'),
        "Digit0" => Key::Char('0'),
        "Minus" => Key::Char('-'),
        "Equal" => Key::Char('='),
        "Backspace" => Key::Special(SpecialKey::Backspace),
        "Tab" => Key::Special(SpecialKey::Tab),
        "KeyQ" => Key::Char('q'),
        "KeyW" => Key::Char('w'),
        "KeyE" => Key::Char('e'),
        "KeyR" => Key::Char('r'),
        "KeyT" => Key::Char('t'),
        "KeyY" => Key::Char('y'),
        "KeyU" => Key::Char('u'),
        "KeyI" => Key::Char('i'),
        "KeyO" => Key::Char('o'),
        "KeyP" => Key::Char('p'),
        "BracketLeft" => Key::Char('['),
        "BracketRight" => Key::Char(']'),
        "Enter" => Key::Special(SpecialKey::Enter),
        "CapsLock" => Key::Special(SpecialKey::Caps),
        "KeyA" => Key::Char('a'),
        "KeyS" => Key::Char('s'),
        "KeyD" => Key::Char('d'),
        "KeyF" => Key::Char('f'),
        "KeyG" => Key::Char('g'),
        "KeyH" => Key::Char('h'),
        "KeyJ" => Key::Char('j'),
        "KeyK" => Key::Char('k'),
        "KeyL" => Key::Char('l'),
        "Semicolon" => Key::Char(';'),
        "Quote" => Key::Char('\''),
        "Backslash" => Key::Char('\\'),
        "ShiftLeft" => Key::Special(SpecialKey::Shift),
        "IntlBackslash" => Key::Char('\\'),
        "KeyZ" => Key::Char('z'),
        "KeyX" => Key::Char('x'),
        "KeyC" => Key::Char('c'),
        "KeyV" => Key::Char('v'),
        "KeyB" => Key::Char('b'),
        "KeyN" => Key::Char('n'),
        "KeyM" => Key::Char('m'),
        "Comma" => Key::Char(','),
        "Period" => Key::Char('.'),
        "Slash" => Key::Char('/'),
        "ShiftRight" => Key::Special(SpecialKey::Shift),
        "ControlLeft" => Key::Special(SpecialKey::Ctrl),
        "OSLeft" | "OSRight" | "MetaLeft" | "MetaRight" => Key::Special(SpecialKey::Meta),
        "AltLeft" => Key::Special(SpecialKey::Alt),
        "Space" => Key::Special(SpecialKey::Space),
        "AltRight" => Key::Special(SpecialKey::Alt),
        "ContextMenu" => Key::Special(SpecialKey::Menu),
        "ControlRight" => Key::Special(SpecialKey::Ctrl),
        _ => return Err(format!("Couldn't parse key: {}", s)),
    };

    Ok(key)
}
