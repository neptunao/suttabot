use chrono::{Datelike, NaiveDate, Utc};
use chrono_tz::Tz;
use solunatus::astro::moon::{lunar_phases, LunarPhaseType};
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
}

#[derive(Debug)]
pub enum LookupError {
    Unknown(String),
}

struct CityRow {
    aliases: &'static [&'static str],
    display: &'static str,
    fallback_tz: Tz,
}

const fn row(
    aliases: &'static [&'static str],
    display: &'static str,
    fallback_tz: Tz,
) -> CityRow {
    CityRow {
        aliases,
        display,
        fallback_tz,
    }
}

// Aliases: Russian names first, then English (first ASCII alias is used as the solunatus query).
static CITIES: &[CityRow] = &[
    // Russia, west to east
    row(&["калининград", "kaliningrad"], "Калининград", Tz::Europe__Kaliningrad),
    row(&["москва", "moscow", "msk"], "Москва", Tz::Europe__Moscow),
    row(&["санкт-петербург", "спб", "saint petersburg"], "Санкт-Петербург", Tz::Europe__Moscow),
    row(&["казань", "kazan"], "Казань", Tz::Europe__Moscow),
    row(&["нижний новгород", "nizhny novgorod"], "Нижний Новгород", Tz::Europe__Moscow),
    row(&["ростов-на-дону", "rostov-on-don"], "Ростов-на-Дону", Tz::Europe__Moscow),
    row(&["краснодар", "krasnodar"], "Краснодар", Tz::Europe__Moscow),
    row(&["воронеж", "voronezh"], "Воронеж", Tz::Europe__Moscow),
    row(&["сочи", "sochi"], "Сочи", Tz::Europe__Moscow),
    row(&["самара", "samara"], "Самара", Tz::Europe__Samara),
    row(&["екатеринбург", "yekaterinburg", "ekaterinburg"], "Екатеринбург", Tz::Asia__Yekaterinburg),
    row(&["уфа", "ufa"], "Уфа", Tz::Asia__Yekaterinburg),
    row(&["челябинск", "chelyabinsk"], "Челябинск", Tz::Asia__Yekaterinburg),
    row(&["пермь", "perm"], "Пермь", Tz::Asia__Yekaterinburg),
    row(&["тюмень", "tyumen"], "Тюмень", Tz::Asia__Yekaterinburg),
    row(&["омск", "omsk"], "Омск", Tz::Asia__Omsk),
    row(&["новосибирск", "novosibirsk"], "Новосибирск", Tz::Asia__Novosibirsk),
    row(&["томск", "tomsk"], "Томск", Tz::Asia__Novosibirsk),
    row(&["красноярск", "krasnoyarsk"], "Красноярск", Tz::Asia__Krasnoyarsk),
    row(&["иркутск", "irkutsk"], "Иркутск", Tz::Asia__Irkutsk),
    row(&["якутск", "yakutsk"], "Якутск", Tz::Asia__Yakutsk),
    row(&["владивосток", "vladivostok"], "Владивосток", Tz::Asia__Vladivostok),
    row(&["хабаровск", "khabarovsk"], "Хабаровск", Tz::Asia__Vladivostok),
    row(&["магадан", "magadan"], "Магадан", Tz::Asia__Magadan),
    row(&["петропавловск-камчатский", "petropavlovsk"], "Петропавловск-Камчатский", Tz::Asia__Kamchatka),
    // Europe
    row(&["лондон", "london"], "Лондон", Tz::Europe__London),
    row(&["дублин", "dublin"], "Дублин", Tz::Europe__Dublin),
    row(&["париж", "paris"], "Париж", Tz::Europe__Paris),
    row(&["мадрид", "madrid"], "Мадрид", Tz::Europe__Madrid),
    row(&["лиссабон", "lisbon"], "Лиссабон", Tz::Europe__Lisbon),
    row(&["берлин", "berlin"], "Берлин", Tz::Europe__Berlin),
    row(&["амстердам", "amsterdam"], "Амстердам", Tz::Europe__Amsterdam),
    row(&["рим", "rome"], "Рим", Tz::Europe__Rome),
    row(&["вена", "vienna"], "Вена", Tz::Europe__Vienna),
    row(&["прага", "prague"], "Прага", Tz::Europe__Prague),
    row(&["варшава", "warsaw"], "Варшава", Tz::Europe__Warsaw),
    row(&["будапешт", "budapest"], "Будапешт", Tz::Europe__Budapest),
    row(&["стокгольм", "stockholm"], "Стокгольм", Tz::Europe__Stockholm),
    row(&["хельсинки", "helsinki"], "Хельсинки", Tz::Europe__Helsinki),
    row(&["афины", "athens"], "Афины", Tz::Europe__Athens),
    row(&["минск", "minsk"], "Минск", Tz::Europe__Minsk),
    row(&["киев", "київ", "kyiv", "kiev"], "Киев", Tz::Europe__Kyiv),
    row(&["рига", "riga"], "Рига", Tz::Europe__Riga),
    row(&["вильнюс", "vilnius"], "Вильнюс", Tz::Europe__Vilnius),
    row(&["таллин", "таллинн", "tallinn"], "Таллин", Tz::Europe__Tallinn),
    // USA (one per timezone)
    row(&["нью-йорк", "нью йорк", "new york", "nyc"], "Нью-Йорк", Tz::America__New_York),
    row(&["чикаго", "chicago"], "Чикаго", Tz::America__Chicago),
    row(&["денвер", "denver"], "Денвер", Tz::America__Denver),
    row(&["лос-анджелес", "лос анджелес", "los angeles", "la"], "Лос-Анджелес", Tz::America__Los_Angeles),
    row(&["анкоридж", "anchorage"], "Анкоридж", Tz::America__Anchorage),
    row(&["гонолулу", "honolulu"], "Гонолулу", Tz::Pacific__Honolulu),
    // Buddhist countries
    row(&["коломбо", "colombo"], "Коломбо", Tz::Asia__Colombo),
    row(&["канди", "kandy"], "Канди", Tz::Asia__Colombo),
    row(&["бангкок", "bangkok"], "Бангкок", Tz::Asia__Bangkok),
    row(&["чиангмай", "чианг май", "chiang mai"], "Чиангмай", Tz::Asia__Bangkok),
    row(&["янгон", "yangon", "rangoon"], "Янгон", Tz::Asia__Yangon),
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
        });
    }

    let row = find_row(&key);

    let en_query: Option<String> = row
        .and_then(first_ascii_alias)
        .map(title_case)
        .or_else(|| {
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
                return Ok(ResolvedLocation { tz, label });
            }
        }
    }

    if let Some(r) = row {
        return Ok(ResolvedLocation {
            tz: r.fallback_tz,
            label: r.display.to_string(),
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
        out.push_str(&format!(
            "{} {} — {}",
            phase.emoji(),
            format_date(*date),
            phase.ru()
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
        };
        let events = upcoming_uposathas(now, &loc, 30);
        assert!(!events.is_empty(), "should have at least one phase in 30 days");
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
}
