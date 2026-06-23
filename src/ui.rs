//! Drawing the machine. A box-drawing frame with single-width console glyphs (card
//! suits, `7`, `$`), centered on the screen.
//!
//! Each reel cell is 5 columns and the whole frame is FRAME_WIDTH (19). The other
//! lines are plain ASCII, so their width is just the character count. We track each
//! line's display width as we build it and centre by that - measuring the strings
//! directly would trip over the colour codes.

use crate::slot::{Machine, Symbol};

const FRAME_WIDTH: usize = 19; // "┌─────┬─────┬─────┐"
const RESET: &str = "\x1b[0m";

/// Tips for the bottom of the screen. Pick the mood: the machine is not your friend.
pub const QUOTES: &[&str] = &[
    "\"You don't gamble to win. You gamble so you can gamble tomorrow.\"",
    "\"The house always wins. You're just here to keep it company.\"",
    "\"Quit while you're ahead. (You won't.)\"",
    "\"It's always the next spin. The next spin is always the one.\"",
    "\"Scared money never made anybody rich.\"",
    "\"The only winning move is not to play. Bit late for that.\"",
    "\"Down is just up that hasn't happened yet.\"",
    "\"Luck is a talent. You weren't born with it.\"",
    "\"The lights are pretty. That's the entire trick.\"",
    "\"One more spin never hurt anybody. Press SPACE.\"",
];

struct Line {
    text: String,
    width: usize,
}

/// Build the whole screen. `pos` is each reel's current strip position; the middle
/// row of the window is the payline. `status` is the one-line message under the box.
pub fn render(
    m: &Machine,
    pos: &[usize; 3],
    spins: u64,
    status: &str,
    quote: &str,
    cols: usize,
    rows: usize,
) -> String {
    let body = vec![
        ascii_bold("HIT  7 7 7  TO BOOT"),
        blank(),
        plain("┌─────┬─────┬─────┐", FRAME_WIDTH),
        reel_row(window(m, pos, -1), false),
        reel_row(window(m, pos, 0), true),
        reel_row(window(m, pos, 1), false),
        plain("└─────┴─────┴─────┘", FRAME_WIDTH),
        blank(),
        ascii(status),
        ascii(&format!("spins: {spins}")),
    ];
    layout(&body, &dim(quote), cols, rows)
}

/// The three symbols on a given window row (offset -1 = above the line, 0 = on it).
fn window(m: &Machine, pos: &[usize; 3], offset: i32) -> [Symbol; 3] {
    let mut row = [Symbol::Seven; 3];
    for (i, slot) in row.iter_mut().enumerate() {
        let len = m.reel_len(i) as i32;
        let p = ((pos[i] as i32 + offset).rem_euclid(len)) as usize;
        *slot = m.symbol(i, p);
    }
    row
}

// The payline row is drawn with double bars and each symbol in its own colour; the
// off-line rows are dimmed so a near-miss seven still reads clearly.
fn reel_row(syms: [Symbol; 3], payline: bool) -> Line {
    let bar = if payline { "║" } else { "│" };
    let mut text = String::from(bar);
    for sym in syms {
        let color = if payline { sym.color() } else { "\x1b[2m" };
        text.push_str("  ");
        text.push_str(color);
        text.push_str(sym.glyph());
        text.push_str(RESET);
        text.push_str("  ");
        text.push_str(bar);
    }
    Line {
        text,
        width: FRAME_WIDTH,
    }
}

/// One-line summary for the scripted (non-TTY) path - used by the test harness.
pub fn plain_line(line: [Symbol; 3]) -> String {
    let tag = if Machine::is_jackpot(line) {
        "   <-- JACKPOT"
    } else {
        ""
    };
    format!(
        "spin:  {}  {}  {}{}",
        line[0].glyph(),
        line[1].glyph(),
        line[2].glyph(),
        tag
    )
}

// Centre the body block and pin the footer (the tip) to the bottom row. We assemble
// the exact rows - top padding, body, filler, footer - then emit at most `rows` of
// them with no trailing newline, each cleared to end of line with "\x1b[K". So the
// in-place redraw never blanks the screen and never scrolls, even on a console too
// short to hold the whole layout.
fn layout(body: &[Line], footer: &Line, cols: usize, rows: usize) -> String {
    let centered = |line: &Line| {
        let pad = cols.saturating_sub(line.width) / 2;
        format!("{}{}", " ".repeat(pad), line.text)
    };

    let mut lines: Vec<String> = Vec::new();
    let usable = rows.saturating_sub(2); // reserve the bottom for the footer
    let top = usable.saturating_sub(body.len()) / 2;
    for _ in 0..top {
        lines.push(String::new());
    }
    for line in body {
        lines.push(centered(line));
    }
    while lines.len() + 1 < rows {
        lines.push(String::new());
    }
    lines.push(centered(footer));

    // Never write more lines than the screen has, or it would scroll on every frame.
    lines.truncate(rows.max(1));
    let mut out = String::new();
    for (i, line) in lines.iter().enumerate() {
        out.push_str(line);
        out.push_str("\x1b[K");
        if i + 1 < lines.len() {
            out.push_str("\r\n");
        }
    }
    out
}

fn ascii(s: &str) -> Line {
    Line {
        text: s.to_string(),
        width: s.chars().count(),
    }
}

// Wrap text in an SGR code (bold, dim, ...). Width is the visible character count -
// the escape codes don't take up columns.
fn styled(s: &str, sgr: &str) -> Line {
    Line {
        text: format!("{sgr}{s}{RESET}"),
        width: s.chars().count(),
    }
}

fn ascii_bold(s: &str) -> Line {
    styled(s, "\x1b[1m")
}

fn dim(s: &str) -> Line {
    styled(s, "\x1b[2m")
}

fn plain(s: &str, width: usize) -> Line {
    Line {
        text: s.to_string(),
        width,
    }
}

fn blank() -> Line {
    Line {
        text: String::new(),
        width: 0,
    }
}
