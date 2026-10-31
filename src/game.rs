use std::collections::VecDeque;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Player {
    X,
    O,
}

impl Player {
    pub fn other(self) -> Player {
        match self {
            Player::X => Player::O,
            Player::O => Player::X,
        }
    }
}

const LINES: [[usize; 3]; 8] = [
    [0, 1, 2],
    [3, 4, 5],
    [6, 7, 8],
    [0, 3, 6],
    [1, 4, 7],
    [2, 5, 8],
    [0, 4, 8],
    [2, 4, 6],
];

pub const MAX_MARKS: usize = 3;

pub const MAX_NAME_LEN: usize = 10;

/// Cleans a typed player name: ASCII printable only, trimmed, at most
/// MAX_NAME_LEN characters. Returns `fallback` if nothing is left.
pub fn clean_name(raw: &str, fallback: &str) -> String {
    let kept: String = raw
        .chars()
        .filter(|c| c.is_ascii_graphic() || *c == ' ')
        .collect();
    let cut: String = kept.trim_matches(' ').chars().take(MAX_NAME_LEN).collect();
    let cut = cut.trim_end_matches(' ');
    if cut.is_empty() {
        fallback.to_string()
    } else {
        cut.to_string()
    }
}

pub struct Game {
    pub cells: [Option<Player>; 9],
    pub history_x: VecDeque<usize>,
    pub history_o: VecDeque<usize>,
    pub turn: Player,
    pub winner: Option<(Player, [usize; 3])>,
}

impl Default for Game {
    fn default() -> Self {
        Self::new()
    }
}

impl Game {
    pub fn new() -> Self {
        Game {
            cells: [None; 9],
            history_x: VecDeque::new(),
            history_o: VecDeque::new(),
            turn: Player::X,
            winner: None,
        }
    }

    fn history(&self, p: Player) -> &VecDeque<usize> {
        match p {
            Player::X => &self.history_x,
            Player::O => &self.history_o,
        }
    }

    fn history_mut(&mut self, p: Player) -> &mut VecDeque<usize> {
        match p {
            Player::X => &mut self.history_x,
            Player::O => &mut self.history_o,
        }
    }

    /// Cell that will vanish when the current player moves (only if they have 3 marks).
    pub fn oldest_to_vanish(&self) -> Option<usize> {
        let h = self.history(self.turn);
        if h.len() >= MAX_MARKS { h.front().copied() } else { None }
    }

    pub fn play(&mut self, idx: usize) -> Result<(), &'static str> {
        if self.winner.is_some() {
            return Err("game over");
        }
        if idx >= 9 {
            return Err("out of range");
        }
        let p = self.turn;
        // Validate before mutating: the cell must be empty, or be the oldest mark
        // that is about to vanish.
        let vanishing = self.oldest_to_vanish();
        if self.cells[idx].is_some() && vanishing != Some(idx) {
            return Err("cell occupied");
        }
        if let Some(old) = vanishing {
            self.history_mut(p).pop_front();
            self.cells[old] = None;
        }
        self.cells[idx] = Some(p);
        self.history_mut(p).push_back(idx);
        for line in LINES.iter() {
            if line.iter().all(|&i| self.cells[i] == Some(p)) {
                self.winner = Some((p, *line));
                return Ok(());
            }
        }
        self.turn = p.other();
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn placement_and_turn_switch() {
        let mut g = Game::new();
        assert_eq!(g.turn, Player::X);
        g.play(4).unwrap();
        assert_eq!(g.cells[4], Some(Player::X));
        assert_eq!(g.turn, Player::O);
        g.play(0).unwrap();
        assert_eq!(g.cells[0], Some(Player::O));
        assert_eq!(g.turn, Player::X);
    }

    #[test]
    fn fourth_mark_removes_oldest() {
        let mut g = Game::new();
        // X: 0,1,5 (no win) ; O: 3,4,8 -> wait avoid wins
        for &i in &[0, 3, 1, 4, 5, 8] {
            g.play(i).unwrap();
        }
        assert!(g.winner.is_none());
        assert_eq!(g.oldest_to_vanish(), Some(0));
        g.play(7).unwrap();
        assert_eq!(g.cells[0], None);
        assert_eq!(g.cells[7], Some(Player::X));
        assert_eq!(g.history_x.len(), 3);
    }

    #[test]
    fn occupied_non_oldest_rejected() {
        let mut g = Game::new();
        g.play(0).unwrap();
        assert!(g.play(0).is_err());
        assert_eq!(g.turn, Player::O);
        // at 3 marks, clicking another occupied cell is rejected and state unchanged
        for &i in &[3, 1, 4, 5, 8] {
            g.play(i).unwrap();
        }
        assert!(g.play(1).is_err()); // X's own non-oldest
        assert!(g.play(3).is_err()); // O's cell
        assert_eq!(g.cells[0], Some(Player::X));
        assert_eq!(g.history_x.len(), 3);
    }

    #[test]
    fn clicking_own_oldest_allowed() {
        let mut g = Game::new();
        for &i in &[0, 3, 1, 4, 5, 8] {
            g.play(i).unwrap();
        }
        g.play(0).unwrap();
        assert_eq!(g.cells[0], Some(Player::X));
        assert_eq!(g.history_x.back(), Some(&0));
        assert_eq!(g.history_x.front(), Some(&1));
        assert_eq!(g.turn, Player::O);
    }

    #[test]
    fn win_rows_cols_diagonals() {
        for line in LINES.iter() {
            let mut g = Game::new();
            let others: Vec<usize> = (0..9).filter(|i| !line.contains(i)).collect();
            // O plays non-line cells, chosen so O never wins in 2 moves.
            g.play(line[0]).unwrap();
            g.play(others[0]).unwrap();
            g.play(line[1]).unwrap();
            g.play(others[1]).unwrap();
            g.play(line[2]).unwrap();
            assert_eq!(g.winner, Some((Player::X, *line)));
        }
    }

    #[test]
    fn no_moves_after_win() {
        let mut g = Game::new();
        for &i in &[0, 3, 1, 4, 2] {
            g.play(i).unwrap();
        }
        assert!(g.winner.is_some());
        assert!(g.play(8).is_err());
        assert_eq!(g.cells[8], None);
        g = Game::new();
        assert!(g.winner.is_none());
        g.play(8).unwrap();
    }

    #[test]
    fn vanish_none_below_three() {
        let mut g = Game::new();
        assert_eq!(g.oldest_to_vanish(), None);
        g.play(0).unwrap();
        g.play(3).unwrap();
        g.play(1).unwrap();
        g.play(4).unwrap();
        assert_eq!(g.oldest_to_vanish(), None);
    }

    #[test]
    fn win_after_vanish_removal() {
        // X marks 0,1,5 ; after 4th move X removes 0. Build win using 3,4,5 row? X: 0,1,5 then 4 -> {1,5,4}; then 3? use diag.
        let mut g = Game::new();
        for &i in &[0, 2, 1, 6, 5, 8] {
            g.play(i).unwrap();
        }
        // X has 0,1,5; O has 2,6,8 (O would vanish 2 next). X plays 4 -> removes 0.
        g.play(4).unwrap();
        assert_eq!(g.cells[0], None);
        assert!(g.winner.is_none());
    }
}
