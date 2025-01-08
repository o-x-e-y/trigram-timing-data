mod mapping;
mod trigram_patterns;
mod with_dof;

use mapping::*;
use trigram_patterns::TRIGRAM_COMBINATIONS;

use std::{collections::HashMap, fs::File, io::Read, path::Path};

use serde::{Deserialize, Serialize};
use serde_with::{serde_as, serde_conv};

serde_conv!(
    TrigramAsPos,
    [Pos; 3],
    |trigram: &[Pos; 3]| format!("{},{},{}", trigram[0], trigram[1], trigram[2]),
    |value: String| {
        value
            .split(",")
            .map(str::parse::<Pos>)
            .collect::<Result<Vec<_>, String>>()?
            .try_into()
            .map_err(|_| "Couldn't turn trigram str into pos trigram".to_string())
    }
);

#[serde_as]
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct TrigramData(#[serde_as(as = "HashMap<TrigramAsPos, _>")] HashMap<[Pos; 3], Vec<u16>>);

#[derive(Clone, Debug)]
pub struct MatrixData(HashMap<[usize; 3], Vec<u16>>);

impl TrigramData {
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self, String> {
        let mut f = File::open(path).map_err(|e| e.to_string())?;

        let mut buf = String::with_capacity(f.metadata().unwrap().len() as usize);
        f.read_to_string(&mut buf).map_err(|e| e.to_string())?;

        serde_json::from_str(&buf).map_err(|e| e.to_string())
    }

    pub fn load_multiple<P: AsRef<Path>>(paths: &[P]) -> Result<Self, String> {
        let datas = paths
            .iter()
            .map(|p| TrigramData::load(p).map_err(|e| e.to_string()))
            .collect::<Result<Vec<_>, String>>()?;

        let data = datas
            .into_iter()
            .reduce(|acc, d| acc.combine(d))
            .unwrap_or_default();

        Ok(data)
    }

    pub fn combine(self, other: Self) -> Self {
        let mut res = Self::default();

        for (trigram, mut freqs) in self.0 {
            res.0
                .entry(trigram)
                .and_modify(|f| f.append(&mut freqs))
                .or_insert(freqs);
        }

        for (trigram, mut freqs) in other.0 {
            res.0
                .entry(trigram)
                .and_modify(|f| f.append(&mut freqs))
                .or_insert(freqs);
        }

        res
    }

    pub fn matrix_3x10(self) -> MatrixData {
        let new_data = self
            .0
            .into_iter()
            .filter(|(poss, _)| {
                poss.iter().all(|p| match p.row {
                    1 | 2 => p.col > 0 && p.col <= 10,
                    3 => p.col <= 12,
                    // 4 => p.col == 0 || p.col == 1,
                    _ => false,
                })
            })
            .map(|(poss, v)| {
                let tri = poss
                    .into_iter()
                    .map(|Pos { row, col }| {
                        let Pos { row, col } = match row {
                            1 | 2 => Pos {
                                row: row - 1,
                                col: col - 1,
                            },
                            3 => match col {
                                0 | 1 => Pos { row: 0, col: 2 },
                                n @ (2..=11) => Pos { row: n - 2, col: 2 },
                                12 => Pos { row: 9, col: 2 },
                                _ => unreachable!(),
                            },
                            _ => unreachable!(),
                        };
                        row * 10 + col
                    })
                    .collect::<Vec<_>>()
                    .try_into()
                    .unwrap();
                (tri, v)
            })
            .collect::<HashMap<_, _>>();
        MatrixData(new_data)
    }
}

#[derive(Default, Debug, Clone)]
pub struct Avg {
    mean: u16,
    sd: u16,
    pop: usize,
}

impl Avg {
    pub fn new(data: Vec<u16>) -> Self {
        if data.is_empty() {
            return Self {
                mean: 0,
                sd: 0,
                pop: 0,
            };
        }

        let mean = data.iter().map(|v| *v as f64).sum::<f64>() / data.len() as f64;
        let sd_mean_corr_sum = data.iter().map(|v| (*v as f64 - mean).powi(2)).sum::<f64>();
        let sd = (sd_mean_corr_sum / ((data.len() - 1) as f64)).sqrt();

        let mean = mean as u16;
        let sd = sd as u16;

        Self {
            mean,
            sd,
            pop: data.len(),
        }
    }
}

impl std::fmt::Display for Avg {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "mean: {}  sd: {:>2}  n: {:<4}  wpm: {}",
            self.mean,
            self.sd,
            self.pop,
            60000 / self.mean * 2 / 5
        )
    }
}

#[derive(Debug, Default, Clone)]
pub struct TrigramStatsInter {
    overall: Vec<u16>,
    sfb: Vec<u16>,
    bad_sfb: Vec<u16>,
    sfs: Vec<u16>,
    sft: Vec<u16>,
    sfr: Vec<u16>,
    alternate: Vec<u16>,
    alternate_sfs: Vec<u16>,
    inroll: Vec<u16>,
    outroll: Vec<u16>,
    onehand: Vec<u16>,
    redirect: Vec<u16>,
    redirect_sfs: Vec<u16>,
    bad_redirect: Vec<u16>,
    bad_redirect_sfs: Vec<u16>,
    other: Vec<u16>,
    invalid: Vec<u16>,
}

impl From<TrigramStatsInter> for TrigramStats {
    fn from(stats: TrigramStatsInter) -> Self {
        TrigramStats {
            sfr: Avg::new(stats.sfr),
            overall: Avg::new(stats.overall),
            alternate: Avg::new(stats.alternate),
            sfs: Avg::new(stats.sfs),
            alternate_sfs: Avg::new(stats.alternate_sfs),
            inroll: Avg::new(stats.inroll),
            outroll: Avg::new(stats.outroll),
            onehand: Avg::new(stats.onehand),
            redirect: Avg::new(stats.redirect),
            redirect_sfs: Avg::new(stats.redirect_sfs),
            bad_redirect: Avg::new(stats.bad_redirect),
            bad_redirect_sfs: Avg::new(stats.bad_redirect_sfs),
            sfb: Avg::new(stats.sfb),
            bad_sfb: Avg::new(stats.bad_sfb),
            sft: Avg::new(stats.sft),
            _other: Avg::new(stats.other),
            _invalid: Avg::new(stats.invalid),
        }
    }
}

#[derive(Debug, Default, Clone)]
pub struct TrigramStats {
    overall: Avg,
    sfb: Avg,
    sfr: Avg,
    sfs: Avg,
    bad_sfb: Avg,
    sft: Avg,
    alternate: Avg,
    alternate_sfs: Avg,
    inroll: Avg,
    outroll: Avg,
    onehand: Avg,
    redirect: Avg,
    redirect_sfs: Avg,
    bad_redirect: Avg,
    bad_redirect_sfs: Avg,
    _other: Avg,
    _invalid: Avg,
}

impl std::fmt::Display for TrigramStats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            concat!(
                "Overall:        {}\n\n",
                "Sfb:            {}\n",
                "BadSfb:         {}\n",
                "Sft:            {}\n",
                "Sfr:            {}\n",
                "Sfs:            {}\n\n",
                "Alternate:      {}\n",
                "Alternate Sfs:  {}\n\n",
                "Inroll:         {}\n",
                "Outroll:        {}\n",
                "Onehand:        {}\n\n",
                "Redirect:       {}\n",
                "RedirectSfs:    {}\n",
                "BadRedirect:    {}\n",
                "BadRedirectSfs: {}\n\n",
                // "Other:          {}\n",
                // "Invalid:        {}\n",
            ),
            self.overall,
            self.sfb,
            self.bad_sfb,
            self.sft,
            self.sfr,
            self.sfs,
            self.alternate,
            self.alternate_sfs,
            self.inroll,
            self.outroll,
            self.onehand,
            self.redirect,
            self.redirect_sfs,
            self.bad_redirect,
            self.bad_redirect_sfs,
            // self.other,
            // self.invalid,
        )
    }
}

fn finger(index: usize) -> usize {
    // match (index / 10, index % 10) {
    //     (3, 0) => Finger::LT,
    //     (_, 0) => Finger::LP,
    //     (_, 1) => Finger::LR,
    //     (_, 2) => Finger::LM,
    //     (_, 3) | (_, 4) => Finger::LI,
    //     (_, 5) | (_, 6)=> Finger::RI,
    //     (_, 7) => Finger::RM,
    //     (_, 8) => Finger::RR,
    //     (_, 9) => Finger::RP,
    //     _ => unreachable!()
    // }
    match index % 10 {
        n @ (0..=3) => n,
        n @ (4 | 5) => n - 1,
        n @ (6..=9) => n - 2,
        _ => unreachable!(),
    }
}

fn fingers<const N: usize>(indexes: &[usize; N]) -> [usize; N] {
    indexes
        .iter()
        .map(|f| finger(*f))
        .collect::<Vec<_>>()
        .try_into()
        .unwrap()
}

fn fingers_are_sfs([a, b, c]: &[usize; 3]) -> bool {
    a == c && a != b
}

fn indexes_are_sfr([a, b, c]: &[usize; 3]) -> bool {
    a == b || b == c
}

impl MatrixData {
    fn stats(&self) -> TrigramStats {
        let mut inter = TrigramStatsInter::default();

        for (indexes, vals) in self.0.iter() {
            if indexes_are_sfr(indexes) {
                inter.sfr.extend(vals);
                inter.overall.extend(vals);
                continue;
            }

            let [a, b, c] = fingers(indexes);

            if fingers_are_sfs(&[a, b, c]) {
                inter.sfs.extend(vals)
            }

            inter.overall.extend(vals);

            let combination = (a << 6) | (b << 3) | c;
            let pattern = TRIGRAM_COMBINATIONS[combination];

            use trigram_patterns::TrigramPattern as T;

            match pattern {
                T::Alternate => inter.alternate.extend(vals),
                T::AlternateSfs => inter.alternate_sfs.extend(vals),
                T::Inroll => inter.inroll.extend(vals),
                T::Outroll => inter.outroll.extend(vals),
                T::Onehand => inter.onehand.extend(vals),
                T::Redirect => inter.redirect.extend(vals),
                T::RedirectSfs => inter.redirect_sfs.extend(vals),
                T::BadRedirect => inter.bad_redirect.extend(vals),
                T::BadRedirectSfs => inter.bad_redirect_sfs.extend(vals),
                T::Sfb => inter.sfb.extend(vals),
                T::BadSfb => inter.bad_sfb.extend(vals),
                T::Sft => inter.sft.extend(vals),
                T::Other => inter.other.extend(vals),
            }
        }

        inter.into()
    }
}

fn main() {
    let paths = std::env::args().skip(1).collect::<Vec<_>>();

    let data = TrigramData::load_multiple(&paths).unwrap().matrix_3x10();

    println!("{}", data.stats());
}
