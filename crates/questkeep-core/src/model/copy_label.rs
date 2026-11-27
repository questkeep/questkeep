//! Telling apart copies of one library entry that come from the same store (two Steam copies,
//! several Beat Saber versions from one plugin store): each gets a qualifier shown after the
//! store's name ("Steam · Half-Life 2: Episode One").

/// What is known about a copy for its qualifier.
#[derive(Debug, Clone, Copy)]
pub struct CopyFacts<'a> {
    pub source: &'a str,
    /// The copy's own game name.
    pub name: &'a str,
    pub install_dir: Option<&'a str>,
    pub source_game_id: Option<&'a str>,
}

/// The qualifier of each copy, in the same order: `None` for a copy whose store is in the list
/// once, else what differs between the copies of that store. Tried in turn, and used when it
/// tells every copy of that store apart: the copies' own names, their install folders' last
/// component, their ids in the store; failing all, their place among them ("#2").
pub fn copy_qualifiers(copies: &[CopyFacts<'_>]) -> Vec<Option<String>> {
    let mut qualifiers = vec![None; copies.len()];
    let mut seen: Vec<&str> = Vec::new();
    for copy in copies {
        if seen.contains(&copy.source) {
            continue;
        }
        seen.push(copy.source);
        let group: Vec<usize> = (0..copies.len())
            .filter(|&i| copies[i].source == copy.source)
            .collect();
        if group.len() < 2 {
            continue;
        }
        let members: Vec<&CopyFacts<'_>> = group.iter().map(|&i| &copies[i]).collect();
        let labels = distinct(&members, |c| Some(c.name))
            .or_else(|| distinct(&members, |c| c.install_dir.and_then(last_component)))
            .or_else(|| distinct(&members, |c| c.source_game_id))
            .unwrap_or_else(|| (1..=members.len()).map(|n| format!("#{n}")).collect());
        for (index, label) in group.into_iter().zip(labels) {
            qualifiers[index] = Some(label);
        }
    }
    qualifiers
}

/// Each copy's value of `key`, when every copy has one (not blank) and no two are the same
/// (ignoring case).
fn distinct<'a>(
    copies: &[&CopyFacts<'a>],
    key: impl Fn(&CopyFacts<'a>) -> Option<&'a str>,
) -> Option<Vec<String>> {
    let values: Vec<String> = copies
        .iter()
        .map(|c| key(c).map(str::trim).filter(|v| !v.is_empty()).map(str::to_string))
        .collect::<Option<_>>()?;
    let mut lowered: Vec<String> = values.iter().map(|v| v.to_lowercase()).collect();
    lowered.sort();
    lowered.dedup();
    (lowered.len() == values.len()).then_some(values)
}

/// A folder path's last component; both separators count, so Windows paths work everywhere.
fn last_component(path: &str) -> Option<&str> {
    path.trim_end_matches(['/', '\\'])
        .rsplit(['/', '\\'])
        .next()
        .filter(|part| !part.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn facts<'a>(
        source: &'a str,
        name: &'a str,
        dir: Option<&'a str>,
        id: Option<&'a str>,
    ) -> CopyFacts<'a> {
        CopyFacts {
            source,
            name,
            install_dir: dir,
            source_game_id: id,
        }
    }

    #[test]
    fn stores_there_once_get_no_qualifier() {
        let copies = [
            facts("steam", "Hades", None, Some("1")),
            facts("epic", "Hades", None, Some("2")),
        ];
        assert_eq!(copy_qualifiers(&copies), vec![None, None]);
    }

    #[test]
    fn different_names_tell_copies_apart() {
        let copies = [
            facts("steam", "Half-Life 2", None, Some("220")),
            facts("epic", "Half-Life 2", None, Some("x")),
            facts("steam", "Half-Life 2: Episode One", None, Some("380")),
        ];
        assert_eq!(
            copy_qualifiers(&copies),
            vec![
                Some("Half-Life 2".into()),
                None,
                Some("Half-Life 2: Episode One".into())
            ]
        );
    }

    #[test]
    fn same_names_fall_back_to_the_install_folder() {
        let copies = [
            facts(
                "bsmanager",
                "Beat Saber",
                Some(r"D:\BSManager\BSInstances\1.29.1\"),
                Some("a"),
            ),
            facts(
                "bsmanager",
                "beat saber",
                Some("/games/BSInstances/1.40.0"),
                Some("b"),
            ),
        ];
        assert_eq!(
            copy_qualifiers(&copies),
            vec![Some("1.29.1".into()), Some("1.40.0".into())]
        );
    }

    #[test]
    fn then_the_store_id_then_the_place() {
        let by_id = [
            facts("steam", "Game", Some(r"C:\a\Game"), Some("10")),
            facts("steam", "Game", Some(r"D:\b\Game"), Some("20")),
        ];
        assert_eq!(
            copy_qualifiers(&by_id),
            vec![Some("10".into()), Some("20".into())]
        );
        let by_place = [
            facts("manual", "Game", None, None),
            facts("manual", "Game", None, None),
        ];
        assert_eq!(
            copy_qualifiers(&by_place),
            vec![Some("#1".into()), Some("#2".into())]
        );
    }

    #[test]
    fn a_missing_value_skips_that_step() {
        let copies = [
            facts("steam", "Game", Some(r"C:\Game A"), Some("1")),
            facts("steam", "Game", None, Some("2")),
        ];
        assert_eq!(
            copy_qualifiers(&copies),
            vec![Some("1".into()), Some("2".into())]
        );
    }
}
