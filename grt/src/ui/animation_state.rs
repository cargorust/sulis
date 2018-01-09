use std::slice::Iter;
use std::io::{Error, ErrorKind};
use std::collections::HashMap;

use self::Kind::*;

#[derive(PartialOrd, Ord, Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum Kind {
    Normal,
    Hover,
    Pressed,
    Active,
}

impl Kind {
    pub fn get_text(&self) -> &str {
        match *self {
            Normal => "normal",
            Hover => "hover",
            Pressed => "pressed",
            Active => "active",
        }
    }

    pub fn iter() -> Iter<'static, Kind> {
        static STATES: [Kind; 4] = [
            Normal,
            Hover,
            Pressed,
            Active
        ];
        STATES.iter()
    }

    pub fn find(text: &str) -> Option<Kind> {
        match text {
            "normal" => Some(Normal),
            "hover" => Some(Hover),
            "pressed" => Some(Pressed),
            "active" => Some(Active),
            _ => None,
        }
    }
}

lazy_static! {
    static ref NORMAL: AnimationState = AnimationState::default();
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct AnimationState {
    kinds: Vec<Kind>,
}

impl AnimationState {
    /// Adds the specified kind to this animation state, if it is not
    /// already present.  Removes `Kind::Normal` if it is present, as
    /// it may only ever be in an otherwise empty AnimationState
    pub fn add(&mut self, kind: Kind) {
        self.kinds.retain(|k| *k != Kind::Normal);

        if !self.kinds.contains(&kind) {
            self.kinds.push(kind);
            self.kinds.sort_unstable();
        }
    }

    /// Removes the specified kind from this animation state, if it
    /// is present.  If the AnimationState would be empty after this,
    /// adds `Kind::Normal`
    pub fn remove(&mut self, kind: Kind) {
        self.kinds.retain(|k| *k != kind);
        if self.kinds.len() == 0 {
            self.kinds.push(Kind::Normal);
        }
    }
}

impl AnimationState {
    /// Creates an instance of the default animation state, containing
    /// `Kind::Normal`
    pub fn default() -> AnimationState {
        AnimationState {
            kinds: vec![Kind::Normal]
        }
    }

    /// Creates a new AnimationState containing only the given `kind`
    pub fn with(kind: Kind) -> AnimationState {
        AnimationState {
            kinds: vec![kind]
        }
    }

    /// Creates an animation state by parsing the given string.  The string
    /// consists of any number of `Kind` string identifiers, separated by the
    /// '+' character and optional whitespace.  An empty state (with no valid
    /// Kinds) is not allowed.  `Kind::Normal` may not be present with any
    /// other Kinds.  Duplicate Kinds are not allowed.
    ///
    /// # Examples
    /// ```
    /// use ui::AnimationState;
    /// let state = AnimationState::parse("hover + active");
    /// let state2 = AnimationState::parse("pressed");
    /// ```
    pub fn parse(data: &str) -> Result<AnimationState, Error> {
        let mut kinds: Vec<Kind> = Vec::with_capacity(1);
        for split in data.split('+').map(|s| s.trim()) {
            match Kind::find(split) {
                Some(kind) => AnimationState::validate_push(kind, &mut kinds)?,
                None => return Err(Error::new(
                        ErrorKind::InvalidData,
                        format!("Unable to parse animation state from '{}'", data))),
            };
        }

        if kinds.len() == 0 {
            return Err(Error::new(ErrorKind::InvalidData, "Empty AnimationState string"));
        }

        return Ok(AnimationState {
            kinds,
        })
    }

    fn validate_push(kind: Kind, kinds: &mut Vec<Kind>) -> Result<(), Error> {
        if kinds.contains(&kind) {
            return Err(Error::new(ErrorKind::InvalidData,
                                  format!("Duplicate key '{:?}'", kind)));
        }

        if kind == Kind::Normal && kinds.len() > 0 {
            return Err(Error::new(ErrorKind::InvalidData,
                                  "Normal state cannot be present with others."));
        } else if kinds.contains(&Kind::Normal) {
            return Err(Error::new(ErrorKind::InvalidData,
                                  "Normal state cannot be present with others."));
        }

        kinds.push(kind);
        kinds.sort_unstable();
        Ok(())
    }

    /// Finds the `AnimationState` within the specified mapping that closest
    /// matches `other`, and returns the corresponding `T`.
    /// In order to be simple and efficient, this does not do exhaustive checking.
    /// Rather, it checks a few simple rules, in order:
    /// First, return an exact match if it exists
    /// Next, return the `Kind::Normal` state if it exists
    /// Finally, return any state at random in the mapping
    pub fn find_match<'a, T>(mapping: &'a HashMap<AnimationState, T>,
                         other: &'a AnimationState) -> &'a T {
        if let Some(t) = mapping.get(other) {
            return t;
        }

        if let Some(t) = mapping.get(&NORMAL) {
            return t;
        }

        return mapping.get(mapping.keys().next().unwrap()).unwrap()
    }
}