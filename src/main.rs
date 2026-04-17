mod trigram_patterns;
mod with_dof;

use libdof::prelude::{Finger, Key, Pos};

use std::{collections::HashMap, fs::File, io::Read, path::Path};

use serde::{Deserialize, Serialize};
use serde_with::{serde_as, serde_conv};

use crate::{
    trigram_patterns::{Trigram, TrigramPattern},
    with_dof::Layout,
};

serde_conv!(
    TrigramAsKey,
    [Key; 3],
    |trigram: &[Key; 3]| format!("{},{},{}", trigram[0], trigram[1], trigram[2]),
    |value: String| {
        value
            .split(",")
            .map(|s| with_dof::parse_key(s))
            .collect::<Result<Vec<_>, String>>()?
            .try_into()
            .map_err(|_| "Couldn't turn trigram str into key trigram".to_string())
    }
);

#[serde_as]
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct TrigramData(#[serde_as(as = "HashMap<TrigramAsKey, _>")] HashMap<[Key; 3], Vec<u16>>);

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

    fn stats(&self, layout: &Layout) -> TrigramStats {
        let mut inter = TrigramStatsInter::default();

        for (keys, vals) in self.0.iter() {
            let seq = match layout.finger_seq(keys.clone()) {
                [Some(s1), Some(s2), Some(s3)] => [s1, s2, s3],
                _ => continue,
            };

            if fingers_are_sfs(&seq) {
                inter.sfs.extend(vals)
            }

            inter.overall.extend(vals);

            let trigram = Trigram::new(seq);
            let pattern = trigram.get_trigram_pattern();

            if matches!(
                pattern,
                TrigramPattern::Sfb | TrigramPattern::Sfr | TrigramPattern::Sft
            ) {
                for (finger, _) in &seq {
                    inter.by_finger.entry(*finger).or_default().extend(vals);
                }
            }

            // println!("{}: {}, {:?}\n{}: {}, {:?}\n{}: {}, {:?}\nBecomes: {:?}\n", keys[0], seq[0].0, seq[0].0.hand(), keys[1], seq[1].0, seq[1].0.hand(), keys[2], seq[2].0, seq[2].0.hand(), pattern);

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
                T::Sfr => inter.sfr.extend(vals),
                T::Other => inter.other.extend(vals),
            }
        }

        inter.into()
    }
}

#[derive(Default, Debug, Clone, Copy)]
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
            "{:<4}     {:<4}    {:<4}     {}",
            self.mean,
            self.sd,
            self.pop,
            if self.mean == 0 {
                0
            } else {
                60000 / self.mean * 2 / 5
            }
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
    by_finger: HashMap<Finger, Vec<u16>>,
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
            by_finger: stats
                .by_finger
                .into_iter()
                .map(|(k, v)| (k, Avg::new(v)))
                .collect(),
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
    by_finger: HashMap<Finger, Avg>,
}

impl std::fmt::Display for TrigramStats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let create_row = |avg: Avg| {
            let mean = avg.mean.to_string();
            let sd = avg.sd.to_string();
            let pop = avg.pop.to_string();
            let wpm = if avg.mean == 0 {
                0.to_string()
            } else {
                (60000 / avg.mean * 2 / 5).to_string()
            };

            [mean, sd, pop, wpm]
        };

        let horizontal = |idx: usize| {
            (
                idx,
                tabled::settings::style::HorizontalLine::inherit(tabled::settings::Style::modern()),
            )
        };

        let mut builder = tabled::builder::Builder::new();

        builder.push_record(["mean", "sd", "n", "wpm"]);

        builder.push_record(create_row(self.overall));
        builder.push_record(create_row(self.sfb));
        builder.push_record(create_row(self.bad_sfb));
        builder.push_record(create_row(self.sft));
        builder.push_record(create_row(self.sfr));
        builder.push_record(create_row(self.sfs));
        builder.push_record(create_row(self.alternate));
        builder.push_record(create_row(self.alternate_sfs));
        builder.push_record(create_row(self.inroll));
        builder.push_record(create_row(self.outroll));
        builder.push_record(create_row(self.onehand));
        builder.push_record(create_row(self.redirect));
        builder.push_record(create_row(self.redirect_sfs));
        builder.push_record(create_row(self.bad_redirect));
        builder.push_record(create_row(self.bad_redirect_sfs));

        builder.insert_column(
            0,
            [
                "",
                "Overall",
                "Sfb",
                "BadSfb",
                "Sft",
                "Sfr",
                "Sfs",
                "Alternate",
                "Alternate Sfs",
                "Inroll",
                "Outroll",
                "Onehand",
                "Redirect",
                "RedirectSfs",
                "BadRedirect",
                "BadRedirectSfs",
            ],
        );

        let mut table = builder.build();

        table.with(
            tabled::settings::Style::modern_rounded()
                .remove_horizontal()
                .horizontals([
                    horizontal(1),
                    horizontal(2),
                    horizontal(7),
                    horizontal(9),
                    horizontal(12),
                ]),
        );
        table.with(tabled::settings::Padding::new(1, 2, 0, 0));

        write!(f, "{}", table)?;
        writeln!(f)?;

        let mut fbuilder = tabled::builder::Builder::new();
        fbuilder.push_record(["mean", "sd", "n", "wpm"]);

        for finger in Finger::FINGERS {
            let avg = self.by_finger.get(&finger).copied().unwrap_or_default();
            fbuilder.push_record(create_row(avg));
        }

        fbuilder.insert_column(
            0,
            ["".to_string()]
                .into_iter()
                .chain(Finger::FINGERS.map(|f| f.to_string()))
                .collect::<Vec<_>>(),
        );

        let mut ftable = fbuilder.build();
        ftable.with(tabled::settings::Style::modern_rounded());
        ftable.with(tabled::settings::Padding::new(1, 2, 0, 0));

        write!(f, "{}", ftable)
    }
}

fn fingers_are_sfs([(a, _), (b, _), (c, _)]: &[(Finger, Pos); 3]) -> bool {
    a == c && a != b
}

// fn indexes_are_sfr([a, b, c]: &[usize; 3]) -> bool {
//     a == b || b == c
// }

fn main() {
    let args = std::env::args().skip(1).collect::<Vec<_>>();

    if args.is_empty() {
        println!("Usage: trigram-timing-data <--layout path to layout> <--data path to data> [path to data]...");
        return;
    }

    let maybe_layout = args
        .iter()
        .skip_while(|&arg| arg != "--layout" && arg != "-l")
        .nth(1);

    let layout = match maybe_layout {
        Some(path) => {
            Layout::load(path).unwrap_or_else(|e| panic!("Failed to load layout at {path}: {e}"))
        }
        None => {
            println!("A layout path must be specified like --layout <path>");
            return;
        }
    };

    let data_paths = args
        .iter()
        .skip_while(|&arg| arg != "--data" && arg != "-d")
        .skip(1)
        .take_while(|&arg| arg != "--layout" && arg != "-l")
        .collect::<Vec<_>>();

    let data = TrigramData::load_multiple(&data_paths).unwrap();

    println!("{}", layout.info());
    println!("{}", data.stats(&layout));
}
