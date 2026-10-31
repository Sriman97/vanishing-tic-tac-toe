mod game;
mod net;

use game::{Game, Player};
use macroquad::prelude::*;

const BG: Color = Color::new(0.102, 0.102, 0.18, 1.0);
const BOARD: Color = Color::new(0.137, 0.137, 0.235, 1.0);
const X_COL: Color = Color::new(1.0, 0.42, 0.42, 1.0);
const O_COL: Color = Color::new(0.306, 0.804, 0.769, 1.0);
const GRID: Color = Color::new(0.30, 0.30, 0.45, 1.0);
const HINT: Color = Color::new(0.62, 0.62, 0.74, 1.0);

/// Seconds for a new mark to pop in, a removed mark to fade out, and the win line to draw.
const POP_TIME: f64 = 0.18;
const VANISH_TIME: f64 = 0.30;
const WIN_LINE_TIME: f64 = 0.35;

fn window_conf() -> Conf {
    Conf {
        window_title: "Vanishing Tic Tac Toe".to_owned(),
        window_width: 480,
        window_height: 720,
        high_dpi: true,
        sample_count: 4,
        ..Default::default()
    }
}

fn col(p: Player) -> Color {
    match p {
        Player::X => X_COL,
        Player::O => O_COL,
    }
}

fn name(p: Player) -> &'static str {
    match p {
        Player::X => "X",
        Player::O => "O",
    }
}

/// Blends `c` toward the background instead of using alpha, so overlapping
/// strokes (the crossing of an X, round caps) don't show darker seams.
fn faded(c: Color, amount: f32) -> Color {
    Color::new(
        c.r + (BG.r - c.r) * amount,
        c.g + (BG.g - c.g) * amount,
        c.b + (BG.b - c.b) * amount,
        1.0,
    )
}

/// Ease-out with a small overshoot, for the pop-in of a new mark.
fn ease_out_back(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0) - 1.0;
    1.0 + t * t * (2.7 * t + 1.7)
}

/// A thick line with rounded ends.
fn stroke(x1: f32, y1: f32, x2: f32, y2: f32, thick: f32, c: Color) {
    draw_line(x1, y1, x2, y2, thick, c);
    draw_poly(x1, y1, 32, thick / 2.0, 0.0, c);
    draw_poly(x2, y2, 32, thick / 2.0, 0.0, c);
}

fn ring(cx: f32, cy: f32, r: f32, thick: f32, c: Color) {
    draw_arc(cx, cy, 128, r - thick / 2.0, 0.0, thick, 360.0, c);
}

fn rounded_rect(x: f32, y: f32, w: f32, h: f32, r: f32, c: Color) {
    draw_rectangle(x + r, y, w - 2.0 * r, h, c);
    draw_rectangle(x, y + r, w, h - 2.0 * r, c);
    for (cx, cy) in [(x + r, y + r), (x + w - r, y + r), (x + r, y + h - r), (x + w - r, y + h - r)] {
        draw_poly(cx, cy, 32, r, 0.0, c);
    }
}

fn draw_mark(p: Player, cx: f32, cy: f32, cell: f32, scale: f32, c: Color, thick: f32) {
    let r = cell * 0.26 * scale;
    let thick = thick * scale.min(1.0);
    match p {
        Player::X => {
            stroke(cx - r, cy - r, cx + r, cy + r, thick, c);
            stroke(cx - r, cy + r, cx + r, cy - r, thick, c);
        }
        Player::O => ring(cx, cy, r * 1.1, thick, c),
    }
}

/// Draws text centered on `cx`, rasterized at the screen's real pixel density
/// so it stays sharp on high-DPI (Retina) displays.
fn centered_text(font: Option<&Font>, t: &str, cx: f32, y: f32, size: f32, c: Color) {
    let (font_size, font_scale, font_scale_aspect) = camera_font_scale(size);
    let d = measure_text(t, font, font_size, font_scale);
    draw_text_ex(
        t,
        cx - d.width / 2.0,
        y,
        TextParams {
            font,
            font_size,
            font_scale,
            font_scale_aspect,
            color: c,
            ..Default::default()
        },
    );
}

/// Returns the screen position of a fresh tap/click, if any (one tap = one event).
fn pointer_pressed() -> Option<(f32, f32)> {
    // Macroquad turns each touch start into a left-button press, so this covers
    // taps too. Reading touches() directly drops quick taps: a touch that starts
    // and ends within one frame is only ever seen as Ended.
    if is_mouse_button_pressed(MouseButton::Left) {
        return Some(mouse_position());
    }
    None
}

/// The game plus the animation state that goes with it.
struct Session {
    game: Game,
    /// When each cell's current mark was placed, for the pop-in animation.
    placed_at: [f64; 9],
    /// A mark that just vanished: (cell, owner, time removed).
    ghost: Option<(usize, Player, f64)>,
    won_at: f64,
}

impl Session {
    fn new() -> Self {
        Session {
            game: Game::new(),
            placed_at: [f64::MIN; 9],
            ghost: None,
            won_at: 0.0,
        }
    }

    fn reset(&mut self) {
        *self = Session::new();
    }

    /// Plays `idx` for the player whose turn it is. Returns false if the move is illegal.
    fn play(&mut self, idx: usize, now: f64) -> bool {
        let mover = self.game.turn;
        let vanishing = self.game.oldest_to_vanish();
        if self.game.play(idx).is_err() {
            return false;
        }
        self.placed_at[idx] = now;
        if let Some(old) = vanishing.filter(|&old| old != idx) {
            self.ghost = Some((old, mover, now));
        }
        if self.game.winner.is_some() {
            self.won_at = now;
        }
        true
    }
}

#[macroquad::main(window_conf)]
async fn main() {
    let font = load_ttf_font_from_bytes(include_bytes!("../assets/Inter-SemiBold.otf")).ok();
    let mut s = Session::new();
    // Online play: the side this device controls, and whether we were connected last frame.
    let mut me: Option<Player> = None;
    let mut was_online = false;

    loop {
        let now = get_time();
        clear_background(BG);
        let (sw, sh) = (screen_width(), screen_height());
        // Header (title + hint) above a square board, the whole block centered.
        // The board takes whatever height is left, so wide and short windows
        // shrink it instead of pushing it off the screen.
        let unit = sw.min(sh);
        let ts = (unit * 0.09).max(22.0);
        let hs = (unit * 0.04).max(13.0);
        let header = ts * 1.25 + hs * 2.2;
        let gap = unit * 0.03;
        let side = (sw * 0.92).min(sh - header - gap - unit * 0.06).max(60.0);
        let top = ((sh - (header + gap + side)) / 2.0).max(0.0);
        let title_y = top + ts;
        let bx = (sw - side) / 2.0;
        let by = top + header + gap;
        let cell = side / 3.0;
        let thick = (side * 0.04).max(4.0);
        let center = |i: usize| {
            (
                bx + (i % 3) as f32 * cell + cell / 2.0,
                by + (i / 3) as f32 * cell + cell / 2.0,
            )
        };

        // Connection changes: a fresh online game starts from an empty board.
        let online = net::connected();
        if online && !was_online {
            s.reset();
            me = net::role();
        } else if !online {
            me = None;
        }
        was_online = online;

        // Moves from the other player.
        while online {
            match net::recv() {
                Some(m) if m < 9 => {
                    if Some(s.game.turn) != me {
                        s.play(m as usize, now);
                    }
                }
                Some(net::REMATCH) => {
                    if s.game.winner.is_some() {
                        s.reset();
                        me = me.map(Player::other);
                    }
                }
                Some(_) => {}
                None => break,
            }
        }

        // Input
        if let Some((px, py)) = pointer_pressed() {
            if s.game.winner.is_some() {
                s.reset();
                if online {
                    // Swap sides so the same player doesn't always start.
                    me = me.map(Player::other);
                    net::send(net::REMATCH);
                }
            } else if px >= bx && px < bx + side && py >= by && py < by + side {
                let c = ((px - bx) / cell) as usize;
                let r = ((py - by) / cell) as usize;
                let idx = r.min(2) * 3 + c.min(2);
                if !online || Some(s.game.turn) == me {
                    if s.play(idx, now) && online {
                        net::send(idx as u8);
                    }
                }
            }
        }

        // Board
        rounded_rect(bx, by, side, side, side * 0.06, BOARD);
        let inset = cell * 0.18;
        for i in 1..3 {
            let o = i as f32 * cell;
            stroke(bx + o, by + inset, bx + o, by + side - inset, thick * 0.35, GRID);
            stroke(bx + inset, by + o, bx + side - inset, by + o, thick * 0.35, GRID);
        }

        // Mark that just vanished: shrinks and fades out.
        if let Some((i, p, t0)) = s.ghost {
            let t = ((now - t0) / VANISH_TIME) as f32;
            if t >= 1.0 {
                s.ghost = None;
            } else {
                let (cx, cy) = center(i);
                draw_mark(p, cx, cy, cell, 1.0 - 0.4 * t, faded(col(p), 0.6 + 0.4 * t), thick);
            }
        }

        // Marks. The oldest mark of the player to move pulses faintly as a warning.
        let fading = if s.game.winner.is_none() { s.game.oldest_to_vanish() } else { None };
        let pulse = 0.5 + 0.5 * (now * 4.0).sin() as f32;
        for (i, c) in s.game.cells.iter().enumerate() {
            if let Some(p) = *c {
                let (cx, cy) = center(i);
                let scale = ease_out_back(((now - s.placed_at[i]) / POP_TIME) as f32);
                let color = if fading == Some(i) {
                    faded(col(p), 0.55 + 0.15 * pulse)
                } else if s.game.winner.is_some_and(|(_, line)| !line.contains(&i)) {
                    faded(col(p), 0.6)
                } else {
                    col(p)
                };
                draw_mark(p, cx, cy, cell, scale, color, thick);
            }
        }

        // Winning line, drawn from end to end.
        if let Some((_, line)) = s.game.winner {
            let t = (((now - s.won_at) / WIN_LINE_TIME) as f32).clamp(0.0, 1.0);
            let t = 1.0 - (1.0 - t) * (1.0 - t);
            let (a, b) = (center(line[0]), center(line[2]));
            let (dx, dy) = (b.0 - a.0, b.1 - a.1);
            let len = (dx * dx + dy * dy).sqrt();
            let (ux, uy) = (dx / len * cell * 0.38, dy / len * cell * 0.38);
            let (sx, sy) = (a.0 - ux, a.1 - uy);
            let (ex, ey) = (b.0 + ux, b.1 + uy);
            stroke(sx, sy, sx + (ex - sx) * t, sy + (ey - sy) * t, thick * 0.9, WHITE);
        }

        let names = [
            game::clean_name(&net::raw_name(Player::X), "X"),
            game::clean_name(&net::raw_name(Player::O), "O"),
        ];
        let pname = |p: Player| names[if p == Player::X { 0 } else { 1 }].clone();

        // Text
        let (title, tc) = match s.game.winner {
            Some((p, _)) if online => {
                (if Some(p) == me { "You win!" } else { "You lose" }.to_owned(), col(p))
            }
            Some((p, _)) => (format!("{} wins!", pname(p)), col(p)),
            None if online => {
                let t = if Some(s.game.turn) == me { "Your turn" } else { "Opponent's turn" };
                (t.to_owned(), col(s.game.turn))
            }
            None => (format!("{}'s turn", pname(s.game.turn)), col(s.game.turn)),
        };
        centered_text(font.as_ref(), &title, sw / 2.0, title_y, ts, tc);
        // Kept short: a long title runs off the edge of narrow phone screens.
        let hint = match (s.game.winner.is_some(), me) {
            (true, _) if online => "Tap for a rematch".to_owned(),
            (true, _) => "Tap to play again".to_owned(),
            (false, Some(p)) => format!("You are {} - oldest of 3 marks vanishes", name(p)),
            (false, None) => "Max 3 marks each - the oldest vanishes".to_owned(),
        };
        centered_text(font.as_ref(), &hint, sw / 2.0, title_y + hs * 1.9, hs, HINT);

        next_frame().await;
    }
}
