use std::collections::HashMap;
use std::sync::LazyLock;

use chrono::{Datelike, Duration, NaiveDate, Weekday};
use serde::Serialize;

use crate::capture::settings::{HolidaySetting, Settings};

static RAW: &[u8] = include_bytes!("holidays.bin");
static FESTIVALS_RAW: &[u8] = include_bytes!("festivals.bin");

const MAGIC: &[u8] = b"FHOL";
const FORMAT_VERSION: u8 = 1;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum HolidayKind {
    Public,
    Bank,
    School,
    Optional,
    Observance,
}

impl HolidayKind {
    fn from_byte(b: u8) -> Option<HolidayKind> {
        Some(match b {
            0 => HolidayKind::Public,
            1 => HolidayKind::Bank,
            2 => HolidayKind::School,
            3 => HolidayKind::Optional,
            4 => HolidayKind::Observance,
            _ => return None,
        })
    }

    pub fn is_day_off(self) -> bool {
        matches!(self, HolidayKind::Public | HolidayKind::Bank)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Occurrence {
    pub date: NaiveDate,
    pub offset_minutes: i32,
    pub minutes: u32,
    pub substitute: bool,
    pub kind: Option<HolidayKind>,
}

impl Occurrence {
    pub fn last_date(&self) -> NaiveDate {
        let end = i64::from(self.offset_minutes) + i64::from(self.minutes.max(1)) - 1;
        self.date + Duration::days(end.div_euclid(1440).max(0))
    }
}

#[derive(Clone, Debug)]
pub struct Definition {
    pub id: String,
    pub computed: bool,
    pub moon_sighting: bool,
    pub added: bool,
    pub source: HolidaySource,
    pub names: Vec<(usize, String)>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum HolidaySource {
    #[default]
    DateHolidays,
    Astronomy,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
pub struct HolidayDataInfo {
    pub source: String,
    pub version: String,
    pub integrity: String,
    pub generated: String,
    pub first_year: i32,
    pub last_year: i32,
}

#[derive(Clone, Copy, Debug)]
pub struct Entry {
    pub definition: usize,
    pub series: usize,
    pub kind: HolidayKind,
}

#[derive(Clone, Debug)]
pub struct Region {
    pub code: String,
    pub parent: Option<usize>,
    pub name: String,
    pub zones: Vec<String>,
    pub languages: Vec<usize>,
    pub entries: Vec<Entry>,
}

#[derive(Clone, Debug)]
struct IndexEntry {
    words: Vec<String>,
    definitions: Vec<usize>,
    language: Option<usize>,
}

#[derive(Debug, Default)]
pub struct HolidayData {
    pub info: HolidayDataInfo,
    pub festival_info: HolidayDataInfo,
    pub languages: Vec<String>,
    pub definitions: Vec<Definition>,
    pub regions: Vec<Region>,
    series: Vec<Vec<Occurrence>>,
    index: HashMap<String, Vec<IndexEntry>>,
    codes: HashMap<String, usize>,
    order: Vec<usize>,
}

struct Reader<'a> {
    bytes: &'a [u8],
    pos: usize,
}

impl Reader<'_> {
    fn byte(&mut self) -> Option<u8> {
        let b = *self.bytes.get(self.pos)?;
        self.pos += 1;
        Some(b)
    }

    fn varint(&mut self) -> Option<u64> {
        let mut value = 0u64;
        for shift in (0..64).step_by(7) {
            let b = self.byte()?;
            value |= u64::from(b & 0x7f) << shift;
            if b & 0x80 == 0 {
                return Some(value);
            }
        }
        None
    }

    fn index(&mut self, limit: usize) -> Option<usize> {
        let v = usize::try_from(self.varint()?).ok()?;
        (v < limit).then_some(v)
    }

    fn count(&mut self) -> Option<usize> {
        let v = usize::try_from(self.varint()?).ok()?;
        (v <= self.bytes.len()).then_some(v)
    }

    fn text(&mut self) -> Option<String> {
        let len = self.count()?;
        let end = self.pos.checked_add(len)?;
        let s = std::str::from_utf8(self.bytes.get(self.pos..end)?).ok()?;
        self.pos = end;
        Some(s.to_string())
    }
}

fn decode(bytes: &[u8]) -> Option<HolidayData> {
    if bytes.get(..MAGIC.len())? != MAGIC {
        return None;
    }
    let mut r = Reader {
        bytes,
        pos: MAGIC.len(),
    };
    if r.byte()? != FORMAT_VERSION {
        return None;
    }
    let source = r.text()?;
    let version = r.text()?;
    let integrity = r.text()?;
    let generated = r.text()?;
    let first_year = i32::try_from(r.varint()?).ok()?;
    let last_year = i32::try_from(r.varint()?).ok()?;
    let strings = (0..r.count()?)
        .map(|_| r.text())
        .collect::<Option<Vec<String>>>()?;
    let s = |r: &mut Reader| -> Option<String> { Some(strings[r.index(strings.len())?].clone()) };
    let languages = (0..r.count()?)
        .map(|_| s(&mut r))
        .collect::<Option<Vec<String>>>()?;
    let mut definitions = Vec::new();
    for _ in 0..r.count()? {
        let id = s(&mut r)?;
        let flags = r.byte()?;
        let mut names = Vec::new();
        for _ in 0..r.count()? {
            let lang = r.index(languages.len())?;
            names.push((lang, s(&mut r)?));
        }
        definitions.push(Definition {
            id,
            computed: flags & 1 != 0,
            moon_sighting: flags & 2 != 0,
            added: flags & 4 != 0,
            source: HolidaySource::DateHolidays,
            names,
        });
    }
    let epoch = NaiveDate::from_ymd_opt(1970, 1, 1)?;
    let mut series = Vec::new();
    for _ in 0..r.count()? {
        let mut day = 0i64;
        let mut list = Vec::new();
        for _ in 0..r.count()? {
            let head = r.varint()?;
            day = day.checked_add(i64::try_from(head >> 3).ok()?)?;
            let (offset_minutes, minutes) = if head & 1 != 0 {
                let zigzag = r.varint()?;
                let magnitude = i32::try_from(zigzag >> 1).ok()?;
                let offset = if zigzag & 1 == 0 {
                    magnitude
                } else {
                    -magnitude - 1
                };
                (offset, u32::try_from(r.varint()?).ok()?)
            } else {
                (0, 1440)
            };
            let kind = if head & 4 != 0 {
                Some(HolidayKind::from_byte(r.byte()?)?)
            } else {
                None
            };
            list.push(Occurrence {
                date: epoch.checked_add_signed(Duration::try_days(day)?)?,
                offset_minutes,
                minutes,
                substitute: head & 2 != 0,
                kind,
            });
        }
        series.push(list);
    }
    let mut regions = Vec::new();
    for _ in 0..r.count()? {
        let code = s(&mut r)?;
        let parent = match r.varint()? {
            0 => None,
            p => Some(usize::try_from(p - 1).ok()?),
        };
        let name = s(&mut r)?;
        let zones = (0..r.count()?)
            .map(|_| s(&mut r))
            .collect::<Option<Vec<String>>>()?;
        let region_languages = (0..r.count()?)
            .map(|_| r.index(languages.len()))
            .collect::<Option<Vec<usize>>>()?;
        let mut entries = Vec::new();
        for _ in 0..r.count()? {
            entries.push(Entry {
                definition: r.index(definitions.len())?,
                series: r.index(series.len())?,
                kind: HolidayKind::from_byte(r.byte()?)?,
            });
        }
        regions.push(Region {
            code,
            parent,
            name,
            zones,
            languages: region_languages,
            entries,
        });
    }
    if r.pos != bytes.len()
        || regions
            .iter()
            .any(|g| g.parent.is_some_and(|p| p >= regions.len()))
    {
        return None;
    }
    let codes = regions
        .iter()
        .enumerate()
        .map(|(i, g)| (g.code.clone(), i))
        .collect();
    Some(HolidayData {
        info: HolidayDataInfo {
            source,
            version,
            integrity,
            generated,
            first_year,
            last_year,
        },
        festival_info: HolidayDataInfo::default(),
        languages,
        definitions,
        regions,
        series,
        index: HashMap::new(),
        codes,
        order: Vec::new(),
    })
}

fn merge_festivals(data: &mut HolidayData, festivals: HolidayData) {
    let language_of =
        |data: &mut HolidayData, name: &str| match data.languages.iter().position(|l| l == name) {
            Some(i) => i,
            None => {
                data.languages.push(name.to_string());
                data.languages.len() - 1
            }
        };
    let definition_base = data.definitions.len();
    let series_base = data.series.len();
    for def in festivals.definitions {
        let names = def
            .names
            .into_iter()
            .map(|(l, n)| (language_of(data, &festivals.languages[l]), n))
            .collect();
        data.definitions.push(Definition {
            source: HolidaySource::Astronomy,
            names,
            ..def
        });
    }
    data.series.extend(festivals.series);
    for region in festivals.regions {
        let prefix = format!("{}-", region.code);
        for target in data
            .regions
            .iter_mut()
            .filter(|r| r.code == region.code || r.code.starts_with(&prefix))
        {
            target.entries.extend(region.entries.iter().map(|e| Entry {
                definition: e.definition + definition_base,
                series: e.series + series_base,
                kind: e.kind,
            }));
        }
    }
    data.festival_info = festivals.info;
}

fn load() -> HolidayData {
    let mut data = decode(RAW).unwrap_or_default();
    if let Some(festivals) = decode(FESTIVALS_RAW) {
        merge_festivals(&mut data, festivals);
    }
    data.build_index();
    data.order = data.compute_order();
    data
}

static DATA: LazyLock<HolidayData> = LazyLock::new(load);

pub fn data() -> &'static HolidayData {
    &DATA
}

pub fn holiday_data_info() -> HolidayDataInfo {
    DATA.info.clone()
}

pub fn festival_data_info() -> HolidayDataInfo {
    DATA.festival_info.clone()
}

fn normal_words(text: &str) -> Vec<(String, usize, usize)> {
    let mut out = Vec::new();
    let mut current = String::new();
    let mut start = 0;
    let mut end = 0;
    for (i, c) in text.char_indices() {
        if c.is_alphanumeric() || matches!(c, '\u{0900}'..='\u{0DFF}') {
            if current.is_empty() {
                start = i;
            }
            current.extend(c.to_lowercase());
            end = i + c.len_utf8();
        } else if matches!(c, '\'' | '’' | 'ʼ' | '‘' | '.') && !current.is_empty() {
            end = i + c.len_utf8();
            if c == '.' {
                out.push((std::mem::take(&mut current), start, i));
            }
        } else if !current.is_empty() {
            out.push((std::mem::take(&mut current), start, end));
        }
    }
    if !current.is_empty() {
        out.push((current, start, end));
    }
    out
}

fn key_words(text: &str) -> Vec<String> {
    normal_words(text).into_iter().map(|(w, _, _)| w).collect()
}

const ALIASES: &[(&[&str], &[&str])] = &[
    (
        &["xmas", "x-mas", "christmas", "xmas day", "x-mas day"],
        &["Christmas Day", "Christmas"],
    ),
    (&["xmas eve", "x-mas eve"], &["Christmas Eve"]),
    (
        &["nye", "new years eve", "new year eve"],
        &["New Year's Eve"],
    ),
    (
        &["new year", "new years", "new years day", "new year day"],
        &["New Year's Day", "New Year"],
    ),
    (
        &[
            "valentines",
            "valentine",
            "valentines day",
            "st valentines day",
            "st. valentines day",
            "saint valentines day",
        ],
        &["Valentine's Day"],
    ),
    (
        &[
            "st patricks",
            "st. patricks",
            "st patricks day",
            "st. patricks day",
            "saint patricks day",
            "st paddys",
            "st paddys day",
            "paddys day",
        ],
        &[
            "St. Patrick's Day",
            "Saint Patrick's Day",
            "St Patrick's Day",
        ],
    ),
    (
        &["presidents day", "president day", "presidents' day"],
        &["Washington's Birthday", "Presidents' Day", "Presidents Day"],
    ),
    (
        &[
            "mothers day",
            "mother day",
            "mums day",
            "moms day",
            "mothering sunday",
        ],
        &["Mother's Day", "Mothering Sunday"],
    ),
    (
        &["fathers day", "father day", "dads day"],
        &["Father's Day"],
    ),
    (&["labor day", "labour day"], &["Labor Day", "Labour Day"]),
    (
        &["thanksgiving", "thanksgiving day"],
        &["Thanksgiving Day", "Thanksgiving"],
    ),
    (
        &[
            "black friday",
            "day after thanksgiving",
            "day after thanksgiving day",
        ],
        &[
            "Day after Thanksgiving Day",
            "Day after Thanksgiving",
            "Black Friday",
        ],
    ),
    (
        &["easter", "easter sunday", "easter day"],
        &["Easter Sunday", "Easter"],
    ),
    (
        &["mardi gras", "shrove tuesday", "pancake day"],
        &["Mardi Gras", "Shrove Tuesday", "Pancake Day"],
    ),
    (
        &[
            "mlk day",
            "martin luther king day",
            "martin luther king jr day",
        ],
        &["Martin Luther King Jr. Day"],
    ),
    (&["deepavali", "divali", "lakshmi puja"], &["Diwali"]),
    (&["dhulandi", "dhuleti", "rangwali holi"], &["Holi"]),
    (
        &["vijayadashami", "vijaya dashami", "dasara", "dussera"],
        &["Dussehra"],
    ),
    (
        &["krishna janmashtami", "gokulashtami", "janmashtmi"],
        &["Janmashtami"],
    ),
    (
        &[
            "vinayaka chaturthi",
            "vinayaka chavithi",
            "ganeshotsav",
            "ganesh chathurthi",
        ],
        &["Ganesh Chaturthi"],
    ),
    (
        &[
            "mahashivratri",
            "maha shivratri",
            "mahashivaratri",
            "shivaratri",
            "shivratri",
        ],
        &["Maha Shivaratri"],
    ),
    (&["rakshabandhan"], &["Raksha Bandhan"]),
    (&["rama navami", "ramnavami", "ram navmi"], &["Ram Navami"]),
    (
        &["navaratri", "sharad navratri", "ghatasthapana"],
        &["Navratri"],
    ),
    (&["chaitra navaratri"], &["Chaitra Navratri"]),
    (&["karwa chauth", "karaka chaturthi"], &["Karva Chauth"]),
    (
        &["makara sankranti", "sankranti", "uttarayan"],
        &["Makar Sankranti"],
    ),
    (&["thai pongal"], &["Pongal"]),
    (&["thiruvonam"], &["Onam"]),
    (
        &[
            "eid ul fitr",
            "eid-ul-fitr",
            "id ul fitr",
            "idul fitr",
            "eid al fitr",
            "ramzan id",
            "ramadan eid",
        ],
        &["Eid al-Fitr", "End of Ramadan (Eid al-Fitr)"],
    ),
    (
        &[
            "bakrid",
            "bakri eid",
            "eid ul adha",
            "eid-ul-adha",
            "eid al adha",
            "id ul zuha",
            "idul zuha",
            "idu'l zuha",
        ],
        &["Eid al-Adha", "Feast of the Sacrifice (Eid al-Adha)"],
    ),
    (&["ashura"], &["Muharram"]),
    (
        &[
            "milad un nabi",
            "id e milad",
            "eid e milad",
            "mawlid",
            "eid milad un nabi",
        ],
        &["Milad-un-Nabi"],
    ),
];

const PRIORITY: &[&str] = &["US", "GB", "CA", "AU", "IN", "IE", "NZ", "ZA"];

impl HolidayData {
    fn english(&self) -> Option<usize> {
        self.languages.iter().position(|l| l == "en")
    }

    fn build_index(&mut self) {
        let english = self.english();
        let mut by_english: HashMap<Vec<String>, Vec<usize>> = HashMap::new();
        let mut index: HashMap<String, Vec<IndexEntry>> = HashMap::new();
        let mut add = |words: Vec<String>, definitions: Vec<usize>, language: Option<usize>| {
            let Some(first) = words.first().cloned() else {
                return;
            };
            let bucket = index.entry(first).or_default();
            match bucket
                .iter_mut()
                .find(|e| e.words == words && e.language == language)
            {
                Some(existing) => {
                    for d in definitions {
                        if !existing.definitions.contains(&d) {
                            existing.definitions.push(d);
                        }
                    }
                }
                None => bucket.push(IndexEntry {
                    words,
                    definitions,
                    language,
                }),
            }
        };
        for (i, def) in self.definitions.iter().enumerate() {
            for (lang, name) in &def.names {
                let words = key_words(name);
                if Some(*lang) == english {
                    by_english.entry(words.clone()).or_default().push(i);
                }
                add(words, vec![i], Some(*lang));
            }
        }
        for (variants, targets) in ALIASES {
            let definitions: Vec<usize> = targets
                .iter()
                .flat_map(|t| by_english.get(&key_words(t)).cloned().unwrap_or_default())
                .collect();
            if definitions.is_empty() {
                continue;
            }
            for v in *variants {
                add(key_words(v), definitions.clone(), None);
            }
        }
        self.index = index;
    }

    pub fn region(&self, code: &str) -> Option<usize> {
        self.codes.get(&code.to_ascii_uppercase()).copied()
    }

    pub fn occurrences(&self, entry: &Entry) -> &[Occurrence] {
        self.series.get(entry.series).map_or(&[], Vec::as_slice)
    }

    fn country_order(&self) -> std::iter::Copied<std::slice::Iter<'_, usize>> {
        self.order.iter().copied()
    }

    fn compute_order(&self) -> Vec<usize> {
        let mut order: Vec<usize> = PRIORITY.iter().filter_map(|c| self.region(c)).collect();
        let mut rest: Vec<usize> = (0..self.regions.len())
            .filter(|i| !order.contains(i))
            .collect();
        rest.sort_by_key(|i| {
            (
                self.regions[*i].parent.is_some(),
                self.regions[*i].code.clone(),
            )
        });
        order.extend(rest);
        order
    }

    pub fn region_for_zone(&self, zone: &str) -> Option<usize> {
        let country = self.country_order().find(|i| {
            self.regions[*i].parent.is_none() && self.regions[*i].zones.iter().any(|z| z == zone)
        })?;
        let children: Vec<usize> = (0..self.regions.len())
            .filter(|i| {
                self.regions[*i].parent == Some(country)
                    && self.regions[*i].zones.iter().any(|z| z == zone)
            })
            .collect();
        match children.as_slice() {
            [only] if self.regions[country].zones.len() > 1 => Some(*only),
            _ => Some(country),
        }
    }

    pub fn region_for_locale(&self, locale: &str) -> Option<usize> {
        let tag = locale.split(['-', '_', '.', '@']).nth(1)?;
        (tag.len() == 2 && tag.chars().all(|c| c.is_ascii_alphabetic()))
            .then(|| self.region(tag))
            .flatten()
            .filter(|i| self.regions[*i].parent.is_none())
    }

    fn within_country(&self, region: usize, definitions: &[usize]) -> Option<(usize, usize, bool)> {
        let mut root = region;
        while let Some(p) = self.regions[root].parent {
            root = p;
        }
        let children: Vec<usize> = (0..self.regions.len())
            .filter(|i| self.regions[*i].parent == Some(root))
            .collect();
        let mut counts: HashMap<usize, (usize, usize, usize)> = HashMap::new();
        for &i in &children {
            if let Some(e) = self.entry_for(i, definitions) {
                let slot = counts
                    .entry(self.regions[i].entries[e].series)
                    .or_insert((0, i, e));
                slot.0 += 1;
            }
        }
        let having: usize = counts.values().map(|c| c.0).sum();
        if having == 0 {
            return None;
        }
        let majority = having * 2 >= children.len();
        counts
            .into_values()
            .max_by_key(|(count, i, _)| (*count, std::cmp::Reverse(*i)))
            .map(|(_, i, e)| (i, e, majority))
    }

    fn root_of(&self, region: usize) -> usize {
        let mut root = region;
        while let Some(p) = self.regions[root].parent {
            root = p;
        }
        root
    }

    fn elsewhere(&self, home: Option<usize>, definitions: &[usize]) -> Option<(usize, usize)> {
        let coverage = |r: usize, e: usize| {
            self.occurrences(&self.regions[r].entries[e])
                .iter()
                .filter(|o| !o.substitute)
                .count()
        };
        let best_in = |candidates: &mut dyn Iterator<Item = usize>| {
            candidates
                .filter_map(|r| self.entry_for(r, definitions).map(|e| (r, e)))
                .fold(None, |best: Option<(usize, usize, usize)>, (r, e)| {
                    let c = coverage(r, e);
                    match best {
                        Some((_, _, bc)) if bc >= c => best,
                        _ => Some((r, e, c)),
                    }
                })
                .map(|(r, e, _)| (r, e))
        };
        if let Some(home) = home {
            let root = self.root_of(home);
            let mut country =
                (0..self.regions.len()).filter(move |i| *i != home && self.root_of(*i) == root);
            if let Some(found) = best_in(&mut country) {
                return Some(found);
            }
        }
        best_in(&mut self.country_order())
    }

    fn entry_for(&self, region: usize, definitions: &[usize]) -> Option<usize> {
        self.regions[region]
            .entries
            .iter()
            .position(|e| definitions.contains(&e.definition))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HolidayScope {
    Off,
    Region(Option<usize>),
}

impl HolidayScope {
    pub fn from_settings(settings: &Settings, zone: Option<&str>) -> HolidayScope {
        let data = data();
        match &settings.holidays {
            HolidaySetting::Off => HolidayScope::Off,
            HolidaySetting::Region { country, region } => {
                let code = match region {
                    Some(r) if !r.is_empty() => format!("{country}-{r}"),
                    _ => country.clone(),
                };
                HolidayScope::Region(data.region(&code).or_else(|| data.region(country)))
            }
            HolidaySetting::Auto => HolidayScope::Region(
                settings
                    .locale
                    .as_deref()
                    .and_then(|l| data.region_for_locale(l))
                    .or_else(|| zone.and_then(|z| data.region_for_zone(z))),
            ),
        }
    }

    pub fn is_off(self) -> bool {
        self == HolidayScope::Off
    }

    pub fn region(self) -> Option<usize> {
        match self {
            HolidayScope::Region(r) => r,
            HolidayScope::Off => None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Holiday {
    pub region: usize,
    pub entry: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HolidayReach {
    Home,
    Elsewhere,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HolidayMatch {
    pub holiday: Holiday,
    pub year: Option<i32>,
    pub start: usize,
    pub end: usize,
    pub foreign: bool,
    pub reach: HolidayReach,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum YearlyShape {
    Fixed {
        month: u32,
        day: u32,
    },
    Nth {
        month: u32,
        position: i8,
        weekday: Weekday,
    },
}

impl Holiday {
    fn entry(self) -> Option<&'static Entry> {
        data().regions.get(self.region)?.entries.get(self.entry)
    }

    pub fn definition(self) -> Option<&'static Definition> {
        data().definitions.get(self.entry()?.definition)
    }

    pub fn kind(self) -> Option<HolidayKind> {
        Some(self.entry()?.kind)
    }

    pub fn region_code(self) -> &'static str {
        data()
            .regions
            .get(self.region)
            .map_or("", |r| r.code.as_str())
    }

    pub fn occurrences(self) -> impl Iterator<Item = &'static Occurrence> {
        self.entry()
            .map_or(&[][..], |e| data().occurrences(e))
            .iter()
            .filter(|o| !o.substitute)
    }

    pub fn date_in(self, year: i32) -> Option<NaiveDate> {
        self.occurrences()
            .map(|o| o.date)
            .find(|d| d.year() == year)
    }

    pub fn next_on_or_after(self, today: NaiveDate) -> Option<NaiveDate> {
        self.occurrences().map(|o| o.date).find(|d| *d >= today)
    }

    pub fn name(self) -> String {
        let data = data();
        let english = data.english();
        self.definition().map_or_else(String::new, |d| {
            d.names
                .iter()
                .find(|(l, _)| Some(*l) == english)
                .or_else(|| d.names.first())
                .map(|(_, n)| n.clone())
                .unwrap_or_default()
        })
    }

    pub fn yearly_shape(self) -> Option<YearlyShape> {
        let dates: Vec<NaiveDate> = self.occurrences().map(|o| o.date).collect();
        let first = *dates.first()?;
        let mut years: Vec<i32> = dates.iter().map(Datelike::year).collect();
        years.dedup();
        if years.len() != dates.len() {
            return None;
        }
        if dates
            .iter()
            .all(|d| d.month() == first.month() && d.day() == first.day())
        {
            return Some(YearlyShape::Fixed {
                month: first.month(),
                day: first.day(),
            });
        }
        let weekday = first.weekday();
        let month = first.month();
        let position = |d: &NaiveDate| -> i8 {
            let next_month_day = *d + Duration::days(7);
            if next_month_day.month() != d.month() {
                -1
            } else {
                i8::try_from((d.day() - 1) / 7 + 1).unwrap_or(0)
            }
        };
        let nth = i8::try_from((first.day() - 1) / 7 + 1).unwrap_or(0);
        let same = |d: &NaiveDate| d.month() == month && d.weekday() == weekday;
        if dates
            .iter()
            .all(|d| same(d) && i8::try_from((d.day() - 1) / 7 + 1).unwrap_or(0) == nth)
        {
            return Some(YearlyShape::Nth {
                month,
                position: nth,
                weekday,
            });
        }
        if dates.iter().all(|d| same(d) && position(d) == -1) {
            return Some(YearlyShape::Nth {
                month,
                position: -1,
                weekday,
            });
        }
        None
    }
}

pub fn find_holidays(text: &str, scope: HolidayScope) -> Vec<HolidayMatch> {
    let HolidayScope::Region(region) = scope else {
        return Vec::new();
    };
    let data = data();
    let words = normal_words(text);
    let allowed = |language: Option<usize>| match language {
        None => true,
        Some(l) => {
            Some(l) == data.english()
                || region.is_some_and(|r| data.regions[r].languages.contains(&l))
        }
    };
    let mut out = Vec::new();
    let mut i = 0;
    while i < words.len() {
        let best = data
            .index
            .get(&words[i].0)
            .into_iter()
            .flatten()
            .filter(|e| allowed(e.language))
            .filter(|e| {
                e.words.len() <= words.len() - i
                    && e.words.iter().zip(&words[i..]).all(|(a, b)| *a == b.0)
            })
            .max_by_key(|e| e.words.len());
        let Some(best) = best else {
            i += 1;
            continue;
        };
        let length = best.words.len();
        let start = words[i].1;
        let mut end = words[i + length - 1].2;
        let found = region
            .and_then(|r| {
                data.entry_for(r, &best.definitions)
                    .map(|e| (r, e, HolidayReach::Home))
            })
            .or_else(|| {
                region.and_then(|r| {
                    data.within_country(r, &best.definitions)
                        .map(|(g, e, majority)| {
                            let reach = if majority {
                                HolidayReach::Home
                            } else {
                                HolidayReach::Elsewhere
                            };
                            (g, e, reach)
                        })
                })
            })
            .or_else(|| {
                data.elsewhere(region, &best.definitions)
                    .map(|(g, e)| (g, e, HolidayReach::Elsewhere))
            });
        let Some((r, e, reach)) = found else {
            i += length;
            continue;
        };
        let mut year = None;
        if let Some((w, ws, we)) = words.get(i + length) {
            let gap = &text[end..*ws];
            if w.len() == 4
                && w.chars().all(|c| c.is_ascii_digit())
                && gap.chars().all(|c| c.is_whitespace() || c == ',')
                && gap.len() <= 3
            {
                year = w.parse().ok();
                end = *we;
            }
        }
        out.push(HolidayMatch {
            holiday: Holiday {
                region: r,
                entry: e,
            },
            year,
            reach,
            foreign: reach == HolidayReach::Elsewhere,
            start,
            end,
        });
        i += length + usize::from(year.is_some());
    }
    out
}

pub fn is_day_off(scope: HolidayScope, date: NaiveDate) -> bool {
    let Some(region) = scope.region() else {
        return false;
    };
    let data = data();
    data.regions[region].entries.iter().any(|e| {
        let list = data.occurrences(e);
        let from = list.partition_point(|o| o.date + Duration::days(7) < date);
        list[from..]
            .iter()
            .take_while(|o| o.date <= date)
            .any(|o| o.kind.unwrap_or(e.kind).is_day_off() && date <= o.last_date())
    })
}
