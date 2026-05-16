use chrono::{Datelike, NaiveDate, Utc};
use chrono_tz::Tz;
use solunatus::astro::moon::{lunar_phases, LunarPhaseType};
use solunatus::astro::sun::{solar_event_time, solar_noon, SolarEvent};
use solunatus::astro::Location;
use solunatus::city::CityDatabase;
use std::sync::OnceLock;

static CITY_DB: OnceLock<Option<CityDatabase>> = OnceLock::new();

fn city_db() -> Option<&'static CityDatabase> {
    CITY_DB
        .get_or_init(|| match CityDatabase::load() {
            Ok(db) => Some(db),
            Err(e) => {
                log::error!("Failed to load solunatus CityDatabase: {e}");
                None
            }
        })
        .as_ref()
}

#[derive(Clone, Copy)]
pub enum Phase {
    New,
    FirstQuarter,
    Full,
    LastQuarter,
}

impl Phase {
    fn emoji(self) -> &'static str {
        match self {
            Phase::New => "🌑",
            Phase::FirstQuarter => "🌓",
            Phase::Full => "🌕",
            Phase::LastQuarter => "🌗",
        }
    }

    fn ru(self) -> &'static str {
        match self {
            Phase::New => "Новолуние",
            Phase::FirstQuarter => "Первая четверть",
            Phase::Full => "Полнолуние",
            Phase::LastQuarter => "Последняя четверть",
        }
    }

    fn from_solunatus(p: &LunarPhaseType) -> Self {
        match p {
            LunarPhaseType::NewMoon => Phase::New,
            LunarPhaseType::FirstQuarter => Phase::FirstQuarter,
            LunarPhaseType::FullMoon => Phase::Full,
            LunarPhaseType::LastQuarter => Phase::LastQuarter,
        }
    }
}

pub struct ResolvedLocation {
    pub tz: Tz,
    pub label: String,
    pub lat: f64,
    pub lon: f64,
}

#[derive(Debug)]
pub enum LookupError {
    Unknown(String),
}

struct CityRow {
    aliases: &'static [&'static str],
    display: &'static str,
    fallback_tz: Tz,
    lat: f64,
    lon: f64,
}

const fn row(
    aliases: &'static [&'static str],
    display: &'static str,
    fallback_tz: Tz,
    lat: f64,
    lon: f64,
) -> CityRow {
    CityRow {
        aliases,
        display,
        fallback_tz,
        lat,
        lon,
    }
}

// Aliases: Russian names first, then English (first ASCII alias is used as the solunatus query).
static CITIES: &[CityRow] = &[
    // Russia, west to east
    row(
        &["калининград", "kaliningrad"],
        "Калининград",
        Tz::Europe__Kaliningrad,
        54.71,
        20.51,
    ),
    row(
        &["москва", "moscow", "msk"],
        "Москва",
        Tz::Europe__Moscow,
        55.76,
        37.62,
    ),
    row(
        &["санкт-петербург", "спб", "saint petersburg"],
        "Санкт-Петербург",
        Tz::Europe__Moscow,
        59.93,
        30.32,
    ),
    row(
        &["казань", "kazan"],
        "Казань",
        Tz::Europe__Moscow,
        55.83,
        49.07,
    ),
    row(
        &["нижний новгород", "nizhny novgorod"],
        "Нижний Новгород",
        Tz::Europe__Moscow,
        56.30,
        43.94,
    ),
    row(
        &["ростов-на-дону", "rostov-on-don"],
        "Ростов-на-Дону",
        Tz::Europe__Moscow,
        47.24,
        39.70,
    ),
    row(
        &["краснодар", "krasnodar"],
        "Краснодар",
        Tz::Europe__Moscow,
        45.04,
        38.98,
    ),
    row(
        &["воронеж", "voronezh"],
        "Воронеж",
        Tz::Europe__Moscow,
        51.67,
        39.18,
    ),
    row(&["сочи", "sochi"], "Сочи", Tz::Europe__Moscow, 43.60, 39.73),
    row(
        &["самара", "samara"],
        "Самара",
        Tz::Europe__Samara,
        53.20,
        50.15,
    ),
    row(
        &["екатеринбург", "yekaterinburg", "ekaterinburg"],
        "Екатеринбург",
        Tz::Asia__Yekaterinburg,
        56.85,
        60.61,
    ),
    row(
        &["уфа", "ufa"],
        "Уфа",
        Tz::Asia__Yekaterinburg,
        54.74,
        55.97,
    ),
    row(
        &["челябинск", "chelyabinsk"],
        "Челябинск",
        Tz::Asia__Yekaterinburg,
        55.16,
        61.44,
    ),
    row(
        &["пермь", "perm"],
        "Пермь",
        Tz::Asia__Yekaterinburg,
        58.01,
        56.25,
    ),
    row(
        &["тюмень", "tyumen"],
        "Тюмень",
        Tz::Asia__Yekaterinburg,
        57.15,
        65.53,
    ),
    row(&["омск", "omsk"], "Омск", Tz::Asia__Omsk, 54.99, 73.37),
    row(
        &["новосибирск", "novosibirsk"],
        "Новосибирск",
        Tz::Asia__Novosibirsk,
        54.98,
        82.90,
    ),
    row(&["томск", "tomsk"], "Томск", Tz::Asia__Tomsk, 56.50, 84.97),
    row(
        &["красноярск", "krasnoyarsk"],
        "Красноярск",
        Tz::Asia__Krasnoyarsk,
        56.02,
        92.87,
    ),
    row(
        &["иркутск", "irkutsk"],
        "Иркутск",
        Tz::Asia__Irkutsk,
        52.30,
        104.30,
    ),
    row(
        &["якутск", "yakutsk"],
        "Якутск",
        Tz::Asia__Yakutsk,
        62.03,
        129.73,
    ),
    row(
        &["владивосток", "vladivostok"],
        "Владивосток",
        Tz::Asia__Vladivostok,
        43.12,
        131.89,
    ),
    row(
        &["хабаровск", "khabarovsk"],
        "Хабаровск",
        Tz::Asia__Vladivostok,
        48.48,
        135.07,
    ),
    row(
        &["магадан", "magadan"],
        "Магадан",
        Tz::Asia__Magadan,
        59.56,
        150.81,
    ),
    row(
        &["петропавловск-камчатский", "petropavlovsk"],
        "Петропавловск-Камчатский",
        Tz::Asia__Kamchatka,
        53.05,
        158.65,
    ),
    // Europe
    row(
        &["лондон", "london"],
        "Лондон",
        Tz::Europe__London,
        51.51,
        -0.13,
    ),
    row(
        &["дублин", "dublin"],
        "Дублин",
        Tz::Europe__Dublin,
        53.35,
        -6.26,
    ),
    row(&["париж", "paris"], "Париж", Tz::Europe__Paris, 48.86, 2.35),
    row(
        &["мадрид", "madrid"],
        "Мадрид",
        Tz::Europe__Madrid,
        40.42,
        -3.70,
    ),
    row(
        &["лиссабон", "lisbon"],
        "Лиссабон",
        Tz::Europe__Lisbon,
        38.72,
        -9.14,
    ),
    row(
        &["берлин", "berlin"],
        "Берлин",
        Tz::Europe__Berlin,
        52.52,
        13.41,
    ),
    row(
        &["амстердам", "amsterdam"],
        "Амстердам",
        Tz::Europe__Amsterdam,
        52.37,
        4.90,
    ),
    row(&["рим", "rome"], "Рим", Tz::Europe__Rome, 41.90, 12.50),
    row(
        &["вена", "vienna"],
        "Вена",
        Tz::Europe__Vienna,
        48.21,
        16.37,
    ),
    row(
        &["прага", "prague"],
        "Прага",
        Tz::Europe__Prague,
        50.08,
        14.44,
    ),
    row(
        &["варшава", "warsaw"],
        "Варшава",
        Tz::Europe__Warsaw,
        52.23,
        21.01,
    ),
    row(
        &["будапешт", "budapest"],
        "Будапешт",
        Tz::Europe__Budapest,
        47.50,
        19.04,
    ),
    row(
        &["стокгольм", "stockholm"],
        "Стокгольм",
        Tz::Europe__Stockholm,
        59.33,
        18.07,
    ),
    row(
        &["хельсинки", "helsinki"],
        "Хельсинки",
        Tz::Europe__Helsinki,
        60.17,
        24.94,
    ),
    row(
        &["афины", "athens"],
        "Афины",
        Tz::Europe__Athens,
        37.98,
        23.73,
    ),
    row(
        &["минск", "minsk"],
        "Минск",
        Tz::Europe__Minsk,
        53.90,
        27.56,
    ),
    row(
        &["киев", "київ", "kyiv", "kiev"],
        "Киев",
        Tz::Europe__Kyiv,
        50.45,
        30.52,
    ),
    row(&["рига", "riga"], "Рига", Tz::Europe__Riga, 56.95, 24.11),
    row(
        &["вильнюс", "vilnius"],
        "Вильнюс",
        Tz::Europe__Vilnius,
        54.69,
        25.28,
    ),
    row(
        &["таллин", "таллинн", "tallinn"],
        "Таллин",
        Tz::Europe__Tallinn,
        59.44,
        24.75,
    ),
    // USA (one per timezone)
    row(
        &["нью-йорк", "нью йорк", "new york", "nyc"],
        "Нью-Йорк",
        Tz::America__New_York,
        40.71,
        -74.01,
    ),
    row(
        &["чикаго", "chicago"],
        "Чикаго",
        Tz::America__Chicago,
        41.88,
        -87.63,
    ),
    row(
        &["денвер", "denver"],
        "Денвер",
        Tz::America__Denver,
        39.74,
        -104.99,
    ),
    row(
        &["лос-анджелес", "лос анджелес", "los angeles", "la"],
        "Лос-Анджелес",
        Tz::America__Los_Angeles,
        34.05,
        -118.24,
    ),
    row(
        &["анкоридж", "anchorage"],
        "Анкоридж",
        Tz::America__Anchorage,
        61.22,
        -149.90,
    ),
    row(
        &["гонолулу", "honolulu"],
        "Гонолулу",
        Tz::Pacific__Honolulu,
        21.31,
        -157.86,
    ),
    // Buddhist countries
    row(
        &["коломбо", "colombo"],
        "Коломбо",
        Tz::Asia__Colombo,
        6.93,
        79.86,
    ),
    row(&["канди", "kandy"], "Канди", Tz::Asia__Colombo, 7.29, 80.63),
    row(
        &["бангкок", "bangkok"],
        "Бангкок",
        Tz::Asia__Bangkok,
        13.76,
        100.50,
    ),
    row(
        &["чиангмай", "чианг май", "chiang mai"],
        "Чиангмай",
        Tz::Asia__Bangkok,
        18.79,
        98.99,
    ),
    row(
        &["янгон", "yangon", "rangoon"],
        "Янгон",
        Tz::Asia__Yangon,
        16.87,
        96.20,
    ),
];

fn find_row(key: &str) -> Option<&'static CityRow> {
    CITIES.iter().find(|r| r.aliases.contains(&key))
}

fn first_ascii_alias(r: &CityRow) -> Option<&'static str> {
    r.aliases.iter().find(|a| a.is_ascii()).copied()
}

fn title_case(s: &str) -> String {
    s.split(' ')
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                None => String::new(),
                Some(c) => c.to_uppercase().to_string() + chars.as_str(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn offset_label(tz: Tz) -> String {
    let formatted = Utc::now().with_timezone(&tz).format("%z").to_string();
    // formatted is like "+0300" or "-0500" or "+0530"
    let sign = &formatted[..1];
    let h: i32 = formatted[1..3].parse().unwrap_or(0);
    let m: i32 = formatted[3..5].parse().unwrap_or(0);
    if m == 0 {
        format!("UTC{}{}", sign, h)
    } else {
        format!("UTC{}{}:{:02}", sign, h, m)
    }
}

pub fn resolve_location(raw: &str) -> Result<ResolvedLocation, LookupError> {
    let key = raw.trim().to_lowercase();

    if key.is_empty() {
        return Ok(ResolvedLocation {
            tz: Tz::Europe__Moscow,
            label: "Москва".to_string(),
            lat: 55.76,
            lon: 37.62,
        });
    }

    let row = find_row(&key);

    let en_query: Option<String> = row.and_then(first_ascii_alias).map(title_case).or_else(|| {
        if key.is_ascii() {
            Some(title_case(&key))
        } else {
            None
        }
    });

    if let (Some(q), Some(db)) = (en_query.as_deref(), city_db()) {
        if let Some(city) = db
            .find_exact(q)
            .or_else(|| db.search(q).into_iter().next().map(|(c, _)| c))
        {
            if let Ok(tz) = city.tz.parse::<Tz>() {
                let label = row
                    .map(|r| r.display.to_string())
                    .unwrap_or_else(|| city.name.clone());
                return Ok(ResolvedLocation {
                    tz,
                    label,
                    lat: city.lat,
                    lon: city.lon,
                });
            }
        }
    }

    if let Some(r) = row {
        return Ok(ResolvedLocation {
            tz: r.fallback_tz,
            label: r.display.to_string(),
            lat: r.lat,
            lon: r.lon,
        });
    }

    Err(LookupError::Unknown(raw.to_string()))
}

fn months_to_scan(from: NaiveDate, to: NaiveDate) -> Vec<(i32, u32)> {
    let mut months = Vec::new();
    let mut y = from.year();
    let mut m = from.month();
    loop {
        months.push((y, m));
        if y == to.year() && m == to.month() {
            break;
        }
        if m == 12 {
            y += 1;
            m = 1;
        } else {
            m += 1;
        }
    }
    months
}

pub fn upcoming_uposathas(
    now: chrono::DateTime<Utc>,
    loc: &ResolvedLocation,
    days_ahead: i64,
) -> Vec<(NaiveDate, Phase)> {
    use chrono::TimeZone;
    let now_local = loc.tz.from_utc_datetime(&now.naive_utc());
    let today = now_local.date_naive();
    let end = today + chrono::Duration::days(days_ahead);

    let mut out = Vec::new();
    for (y, m) in months_to_scan(today, end) {
        for ev in lunar_phases(y, m) {
            let local_date = loc
                .tz
                .from_utc_datetime(&ev.datetime.naive_utc())
                .date_naive();
            if local_date >= today && local_date < end {
                out.push((local_date, Phase::from_solunatus(&ev.phase_type)));
            }
        }
    }
    out.sort_by_key(|(d, _)| *d);
    out
}

const RU_MONTHS: &[&str] = &[
    "января",
    "февраля",
    "марта",
    "апреля",
    "мая",
    "июня",
    "июля",
    "августа",
    "сентября",
    "октября",
    "ноября",
    "декабря",
];

const RU_WEEKDAYS: &[&str] = &["пн", "вт", "ср", "чт", "пт", "сб", "вс"];

fn format_date(d: NaiveDate) -> String {
    let month = RU_MONTHS[d.month0() as usize];
    // chrono weekday: Mon=0 ... Sun=6
    let wd = RU_WEEKDAYS[d.weekday().num_days_from_monday() as usize];
    format!("{} {} ({})", d.day(), month, wd)
}

fn solar_info(date: NaiveDate, loc: &ResolvedLocation) -> Option<String> {
    use chrono::TimeZone;
    let location = Location::new(loc.lat, loc.lon).ok()?;
    let dt = loc
        .tz
        .with_ymd_and_hms(date.year(), date.month(), date.day(), 12, 0, 0)
        .single()?;

    let noon = solar_noon(&location, &dt);
    let dawn = solar_event_time(&location, &dt, SolarEvent::CivilDawn);

    let noon_str = noon.format("%H:%M").to_string();
    let dawn_str = dawn
        .map(|d| d.format("%H:%M").to_string())
        .unwrap_or_else(|| "—".to_string());

    Some(format!("полдень {noon_str}, рассвет {dawn_str}"))
}

pub fn format_message(loc: &ResolvedLocation, events: &[(NaiveDate, Phase)]) -> String {
    let offset = offset_label(loc.tz);
    let mut out = format!(
        "Дни Упосатхи на ближайшие 30 дней ({}, {}):\n",
        loc.label, offset
    );
    if events.is_empty() {
        out.push_str("\nНет данных.");
        return out;
    }
    for (date, phase) in events {
        out.push('\n');
        let solar = solar_info(*date, loc)
            .map(|info| format!(": ☀️ {info}"))
            .unwrap_or_default();
        out.push_str(&format!(
            "{} {} — {}{}",
            phase.emoji(),
            format_date(*date),
            phase.ru(),
            solar,
        ));
    }
    out
}

pub fn format_unknown_city_error(input: &str) -> String {
    format!(
        "Город «{}» не найден.\n\nПримеры: Москва, Бангкок, Коломбо, Владивосток, Berlin, New York.\n\nМожно использовать русские или английские названия.",
        input
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    #[test]
    fn resolve_empty_gives_moscow() {
        let loc = resolve_location("").unwrap();
        assert_eq!(loc.tz, Tz::Europe__Moscow);
        assert_eq!(loc.label, "Москва");
    }

    #[test]
    fn resolve_russian_primary() {
        let loc = resolve_location("Москва").unwrap();
        assert_eq!(loc.tz, Tz::Europe__Moscow);
    }

    #[test]
    fn resolve_ascii_alias() {
        let loc = resolve_location("moscow").unwrap();
        assert_eq!(loc.tz, Tz::Europe__Moscow);
    }

    #[test]
    fn resolve_bangkok() {
        let loc = resolve_location("бангкок").unwrap();
        assert_eq!(loc.tz, Tz::Asia__Bangkok);
        assert_eq!(loc.label, "Бангкок");
    }

    #[test]
    fn resolve_spb_alias() {
        let loc = resolve_location("спб").unwrap();
        assert_eq!(loc.tz, Tz::Europe__Moscow);
        assert_eq!(loc.label, "Санкт-Петербург");
    }

    #[test]
    fn resolve_vladivostok() {
        let loc = resolve_location("Владивосток").unwrap();
        assert_eq!(loc.tz, Tz::Asia__Vladivostok);
    }

    #[test]
    fn resolve_unknown_returns_error() {
        assert!(matches!(
            resolve_location("xyz_unknown_city"),
            Err(LookupError::Unknown(_))
        ));
    }

    #[test]
    fn upcoming_uposathas_returns_sorted_dates_in_window() {
        // Use a fixed "now" in early May 2026 UTC
        let now = Utc.with_ymd_and_hms(2026, 5, 1, 0, 0, 0).unwrap();
        let loc = ResolvedLocation {
            tz: Tz::Europe__Moscow,
            label: "Москва".to_string(),
            lat: 55.76,
            lon: 37.62,
        };
        let events = upcoming_uposathas(now, &loc, 30);
        assert!(
            !events.is_empty(),
            "should have at least one phase in 30 days"
        );
        assert!(events.len() >= 4, "should have roughly 4 phases in 30 days");
        // dates must be sorted ascending
        let dates: Vec<_> = events.iter().map(|(d, _)| *d).collect();
        let mut sorted = dates.clone();
        sorted.sort();
        assert_eq!(dates, sorted, "events must be sorted by date");
        // all dates in window
        let today = Tz::Europe__Moscow
            .from_utc_datetime(&now.naive_utc())
            .date_naive();
        let end = today + chrono::Duration::days(30);
        for (d, _) in &events {
            assert!(*d >= today && *d < end, "date {d} out of window");
        }
    }

    #[test]
    fn solar_info_moscow_returns_times() {
        let loc = ResolvedLocation {
            tz: Tz::Europe__Moscow,
            label: "Москва".to_string(),
            lat: 55.76,
            lon: 37.62,
        };
        let date = NaiveDate::from_ymd_opt(2026, 5, 15).unwrap();
        let info = solar_info(date, &loc);
        assert!(info.is_some());
        let s = info.unwrap();
        assert!(s.contains("рассвет"), "should contain рассвет: {s}");
        assert!(s.contains("полдень"), "should contain полдень: {s}");
    }
}
