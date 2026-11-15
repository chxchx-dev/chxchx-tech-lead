//! History graph: assigns each commit a lane, newest first, and draws it as text.
use super::types::{GitCommit, GraphRow};

/// Fills `graph` of every commit. Commits must be in topological order, newest first.
pub fn assign_lanes(commits: &mut [GitCommit]) {
    // Each lane holds the hash of the commit its line is heading to.
    let mut lanes: Vec<Option<String>> = Vec::new();
    for commit in commits.iter_mut() {
        let matching: Vec<usize> = lanes
            .iter()
            .enumerate()
            .filter(|(_, wanted)| wanted.as_deref() == Some(commit.hash.as_str()))
            .map(|(index, _)| index)
            .collect();
        let lane = matching
            .first()
            .copied()
            .unwrap_or_else(|| free_lane(&mut lanes));
        let through: Vec<usize> = lanes
            .iter()
            .enumerate()
            .filter(|(index, wanted)| wanted.is_some() && !matching.contains(index))
            .map(|(index, _)| index)
            .collect();
        let merge_in: Vec<usize> = matching
            .iter()
            .copied()
            .filter(|index| *index != lane)
            .collect();
        for index in &matching {
            lanes[*index] = None;
        }
        let mut parent_lanes = Vec::new();
        for (position, parent) in commit.parents.iter().enumerate() {
            // The first parent always continues on this commit's lane, even when another
            // line already heads to it: both lines then meet at that parent.
            let target = if position == 0 {
                set(&mut lanes, lane, parent);
                lane
            } else {
                match lanes
                    .iter()
                    .position(|wanted| wanted.as_deref() == Some(parent.as_str()))
                {
                    Some(index) => index,
                    None => {
                        let index = free_lane(&mut lanes);
                        set(&mut lanes, index, parent);
                        index
                    }
                }
            };
            parent_lanes.push(target);
        }
        while lanes.last().is_some_and(Option::is_none) {
            lanes.pop();
        }
        let lane_count = [lane]
            .iter()
            .chain(&through)
            .chain(&merge_in)
            .chain(&parent_lanes)
            .max()
            .copied()
            .unwrap_or(0)
            + 1;
        commit.graph = GraphRow {
            lane,
            incoming: !matching.is_empty(),
            through,
            merge_in,
            parent_lanes,
            lane_count,
        };
    }
}

fn free_lane(lanes: &mut Vec<Option<String>>) -> usize {
    match lanes.iter().position(Option::is_none) {
        Some(index) => index,
        None => {
            lanes.push(None);
            lanes.len() - 1
        }
    }
}

fn set(lanes: &mut Vec<Option<String>>, index: usize, hash: &str) {
    if lanes.len() <= index {
        lanes.resize(index + 1, None);
    }
    lanes[index] = Some(hash.to_owned());
}

/// One text cell per lane: `●` the commit, `│` a line passing through, `╯` a line
/// merging in, `╮` a new line starting to the right.
pub fn render(row: &GraphRow, width: usize) -> String {
    let mut cells = Vec::new();
    for lane in 0..width.max(row.lane_count) {
        let cell = if lane == row.lane {
            '●'
        } else if row.through.contains(&lane) {
            '│'
        } else if row.merge_in.contains(&lane) {
            '╯'
        } else if row.parent_lanes.contains(&lane) {
            '╮'
        } else {
            ' '
        };
        cells.push(cell.to_string());
    }
    cells.join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn commit(hash: &str, parents: &[&str]) -> GitCommit {
        GitCommit {
            hash: hash.to_owned(),
            parents: parents.iter().map(|p| (*p).to_owned()).collect(),
            author: String::new(),
            time: 0,
            refs: Vec::new(),
            subject: String::new(),
            graph: GraphRow::default(),
        }
    }

    #[test]
    fn a_straight_history_stays_in_one_lane() {
        let mut commits = vec![commit("c", &["b"]), commit("b", &["a"]), commit("a", &[])];
        assign_lanes(&mut commits);
        assert!(
            commits
                .iter()
                .all(|c| c.graph.lane == 0 && c.graph.lane_count == 1)
        );
        assert!(!commits[0].graph.incoming && commits[1].graph.incoming);
    }

    #[test]
    fn a_merge_opens_a_second_lane_and_closes_it_at_the_fork() {
        // m merges feature (f) into main (b); both come from a.
        let mut commits = vec![
            commit("m", &["b", "f"]),
            commit("f", &["a"]),
            commit("b", &["a"]),
            commit("a", &[]),
        ];
        assign_lanes(&mut commits);
        assert_eq!(commits[0].graph.parent_lanes, [0, 1]);
        assert_eq!(
            commits[1].graph.lane, 1,
            "the feature commit stays on its own lane"
        );
        assert_eq!(commits[1].graph.through, [0]);
        assert_eq!(
            commits[3].graph.merge_in,
            [1],
            "the second line joins at the fork"
        );
        assert_eq!(render(&commits[0].graph, 2), "● ╮");
        assert_eq!(render(&commits[1].graph, 2), "│ ●");
        assert_eq!(render(&commits[3].graph, 2), "● ╯");
    }
}
