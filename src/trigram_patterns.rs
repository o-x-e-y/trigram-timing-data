use libdof::{
    dofinitions::Hand::{self, *},
    prelude::{
        Finger::{self, *},
        Pos,
    },
};

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum TrigramPattern {
    Alternate,
    AlternateSfs,
    Inroll,
    Outroll,
    Onehand,
    Redirect,
    RedirectSfs,
    BadRedirect,
    BadRedirectSfs,
    Sfb,
    BadSfb,
    Sft,
    Sfr,
    Other,
}

pub trait FingerApi {
    fn eq(self, other: Finger) -> bool;

    fn gt(self, other: Finger) -> bool;

    fn lt(self, other: Finger) -> bool;

    fn is_bad(&self) -> bool;
}

impl FingerApi for Finger {
    fn eq(self, other: Finger) -> bool {
        self as u8 == other as u8
    }

    fn gt(self, other: Finger) -> bool {
        self as u8 > other as u8
    }

    fn lt(self, other: Finger) -> bool {
        (self as u8) < (other as u8)
    }

    fn is_bad(&self) -> bool {
        matches!(self, LP | LR | LM | RM | RR | RP)
    }
}

#[derive(Debug)]
pub(crate) struct Trigram {
    f1: Finger,
    f2: Finger,
    f3: Finger,
    p1: Pos,
    p2: Pos,
    p3: Pos,
    h1: Hand,
    h2: Hand,
    h3: Hand,
}

impl std::fmt::Display for Trigram {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}, {}, {}", self.f1, self.f2, self.f3)
    }
}

impl Trigram {
    pub fn new(fps: [(Finger, Pos); 3]) -> Self {
        let [fp1, fp2, fp3] = fps;

        Trigram {
            f1: fp1.0,
            f2: fp2.0,
            f3: fp3.0,
            p1: fp1.1,
            p2: fp2.1,
            p3: fp3.1,
            h1: fp1.0.hand(),
            h2: fp2.0.hand(),
            h3: fp3.0.hand(),
        }
    }

    fn is_sfr(&self) -> bool {
        self.p1 == self.p2 || self.p2 == self.p3
    }

    fn is_alt(&self) -> bool {
        #[allow(clippy::match_like_matches_macro)]
        match (self.h1, self.h2, self.h3) {
            (Left, Right, Left) => true,
            (Right, Left, Right) => true,
            _ => false,
        }
    }

    fn is_sfs(&self) -> bool {
        self.f1 == self.f3
    }

    fn get_alternate(&self) -> TrigramPattern {
        use TrigramPattern::*;

        match self.is_sfs() {
            true => AlternateSfs,
            false => Alternate,
        }
    }

    fn is_roll(&self) -> bool {
        #[allow(clippy::match_like_matches_macro)]
        match (self.h1, self.h2, self.h3) {
            (Left, Left, Right) => true,
            (Right, Left, Left) => true,
            (Right, Right, Left) => true,
            (Left, Right, Right) => true,
            _ => false,
        }
    }

    fn is_inroll(&self) -> bool {
        match (self.h1, self.h2, self.h3) {
            (Left, Left, Right) => self.f1.lt(self.f2),
            (Right, Left, Left) => self.f2.lt(self.f3),
            (Right, Right, Left) => self.f1.gt(self.f2),
            (Left, Right, Right) => self.f2.gt(self.f3),
            _ => unreachable!(),
        }
    }

    fn get_roll(&self) -> TrigramPattern {
        use TrigramPattern::*;

        match self.is_inroll() {
            true => Inroll,
            false => Outroll,
        }
    }

    fn on_one_hand(&self) -> bool {
        #[allow(clippy::match_like_matches_macro)]
        match (self.h1, self.h2, self.h3) {
            (Left, Left, Left) => true,
            (Right, Right, Right) => true,
            _ => false,
        }
    }

    fn is_redir(&self) -> bool {
        (self.f1.lt(self.f2) == self.f2.gt(self.f3)) && self.on_one_hand()
    }

    fn is_bad_redir(&self) -> bool {
        self.is_redir() && self.f1.is_bad() && self.f2.is_bad() && self.f3.is_bad()
    }

    fn has_sfb(&self) -> bool {
        (self.f1 == self.f2 || self.f2.eq(self.f3)) && !self.is_sfr()
    }

    fn is_sft(&self) -> bool {
        self.f1.eq(self.f2) && self.f2.eq(self.f3) && !self.is_sfr()
    }

    fn get_one_hand(&self) -> TrigramPattern {
        use TrigramPattern::*;

        if self.is_sfr() {
            Sfr
        } else if self.is_sft() {
            Sft
        } else if self.has_sfb() {
            BadSfb
        } else if self.is_redir() {
            match (self.is_sfs(), self.is_bad_redir()) {
                (false, false) => Redirect,
                (false, true) => BadRedirect,
                (true, false) => RedirectSfs,
                (true, true) => BadRedirectSfs,
            }
        } else {
            Onehand
        }
    }

    pub fn get_trigram_pattern(&self) -> TrigramPattern {
        if self.is_sfr() {
            TrigramPattern::Sfr
        } else if self.is_alt() {
            self.get_alternate()
        } else if self.on_one_hand() {
            self.get_one_hand()
        } else if self.has_sfb() {
            TrigramPattern::Sfb
        } else if self.is_roll() {
            self.get_roll()
        } else {
            TrigramPattern::Other
        }
    }
}

// fn get_trigram_combinations() -> [TrigramPattern; 512] {
//     let mut combinations: [TrigramPattern; 512] = [TrigramPattern::Other; 512];

//     let mut c3 = 0;
//     while c3 < 8 {
//         let mut c2 = 0;
//         while c2 < 8 {
//             let mut c1 = 0;
//             while c1 < 8 {
//                 let index = c3 * 64 + c2 * 8 + c1;
//                 let trigram = Trigram::new(
//                     Finger::from_usize(c3),
//                     Finger::from_usize(c2),
//                     Finger::from_usize(c1),
//                 );
//                 combinations[index] = trigram.get_trigram_pattern();
//                 c1 += 1;
//             }
//             c2 += 1;
//         }
//         c3 += 1;
//     }
//     combinations
// }

// pub static TRIGRAM_COMBINATIONS: [TrigramPattern; 512] = get_trigram_combinations();
