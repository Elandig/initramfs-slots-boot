//! slots-boot - a slot machine that runs from the initramfs and won't let the boot
//! finish until you spin 7-7-7. Each press of SPACE spins the reels once; they roll
//! and cascade to a stop on their own.

mod rng;
mod slot;
mod term;
mod ui;

use slot::Machine;

// Type this at the slot screen to skip the game and let the boot continue. It's the
// way out if something ever goes wrong, so it's deliberately not shown on screen.
const BYPASS: &[u8] = b"letmeboot";

const FRAME_MS: u64 = 33; // ~30 fps

// Repaint at least this often even when nothing changed, so messages other boot
// services print over us (common in a systemd initramfs) get cleaned up promptly.
const REPAINT_EVERY: u64 = 6;

// Frames from the start of a spin until each reel locks. Staggered, so the reels
// settle left-to-right. A spin always runs to completion - pressing SPACE again while
// it's going does nothing.
const STOP: [u64; 3] = [18, 32, 48];

enum Outcome {
    Won,
    Bypass,
    Ended,
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.iter().any(|a| a == "-h" || a == "--help") {
        print_usage();
        return;
    }

    let tty = term::is_tty();
    let outcome = if tty { play() } else { play_scripted() };

    // Exit 0 always - the only ways out are a win or the bypass, and both should let
    // the boot continue. In scripted mode we print a marker the test harness checks.
    if !tty {
        match outcome {
            Outcome::Won => println!("WON"),
            Outcome::Bypass => println!("BYPASS"),
            Outcome::Ended => {}
        }
    }
}

/// Interactive game loop. Draws in place (no full-screen clear, so it doesn't
/// flicker), drains input without blocking, and runs each spin to completion.
fn play() -> Outcome {
    let raw = match term::RawMode::enable() {
        Ok(r) => r,
        Err(_) => return play_scripted(),
    };
    term::hide_cursor();
    term::clear();
    let (cols, rows) = term::query_size();
    let (cols, rows) = (cols as usize, rows as usize);

    let machine = Machine::new();
    let mut rng = rng::Rng::from_env();
    let mut pos = [
        machine.random_stop(0, &mut rng),
        machine.random_stop(1, &mut rng),
        machine.random_stop(2, &mut rng),
    ];
    let mut spin_start: Option<u64> = None; // None = idle (SPACE starts a spin)
    let mut next_advance = [0u64; 3];
    let mut spins = 0u64;
    let mut buf = Vec::new();
    let mut t = 0u64;
    let mut last_frame = String::new();
    let mut last_paint = 0u64;
    let mut quote = ui::QUOTES[rng.below(ui::QUOTES.len())];

    let outcome = 'game: loop {
        for b in term::read_pending() {
            if matches_phrase(&mut buf, b, BYPASS) {
                break 'game Outcome::Bypass;
            }
            // A spin only starts from idle - mashing SPACE while it's spinning does
            // nothing, by design.
            if is_spin(b) && spin_start.is_none() {
                spin_start = Some(t);
                next_advance = [t, t, t];
            }
        }

        if let Some(start) = spin_start {
            let mut still_spinning = false;
            for i in 0..3 {
                let elapsed = t - start;
                if elapsed < STOP[i] {
                    still_spinning = true;
                    if t >= next_advance[i] {
                        pos[i] = (pos[i] + 1) % machine.reel_len(i);
                        // Slow the roll as the reel nears its stop, for a smooth finish.
                        let remaining = STOP[i] - elapsed;
                        let step = if remaining > 12 {
                            2
                        } else if remaining > 6 {
                            3
                        } else {
                            4
                        };
                        next_advance[i] = t + step;
                    }
                } else if elapsed == STOP[i] {
                    // Reel just stopped: land on a fresh random symbol so the odds hold.
                    pos[i] = machine.random_stop(i, &mut rng);
                }
            }
            if !still_spinning {
                spins += 1;
                let line = [
                    machine.symbol(0, pos[0]),
                    machine.symbol(1, pos[1]),
                    machine.symbol(2, pos[2]),
                ];
                spin_start = None;
                quote = ui::QUOTES[rng.below(ui::QUOTES.len())];
                if Machine::is_jackpot(line) {
                    celebrate(&machine, &pos, spins, cols, rows);
                    break 'game Outcome::Won;
                }
            }
        }

        let status = if spin_start.is_some() {
            "* * *   spinning   * * *"
        } else {
            "press SPACE to spin"
        };
        let frame = ui::render(&machine, &pos, spins, status, quote, cols, rows);
        // Repaint on change, and periodically regardless so boot-message noise printed
        // over us gets clobbered. The redraw is in place, so this doesn't flicker.
        if frame != last_frame || t - last_paint >= REPAINT_EVERY {
            term::draw(&frame);
            last_frame = frame;
            last_paint = t;
        }
        term::sleep_ms(FRAME_MS);
        t += 1;
    };

    term::show_cursor();
    print!("\r\n");
    drop(raw);
    outcome
}

fn celebrate(machine: &Machine, pos: &[usize; 3], spins: u64, cols: usize, rows: usize) {
    let quote = "\"The house let you win. This once. Now boot.\"";
    let show =
        |status: &str| term::draw(&ui::render(machine, pos, spins, status, quote, cols, rows));
    show("*** J A C K P O T ***");
    term::sleep_ms(500);
    show("the gate opens - booting...");
    term::sleep_ms(1600);
}

/// Non-interactive path: one spin per key, plain-text output. This is what the Docker
/// and QEMU tests drive, and what runs if we can't get a real terminal.
fn play_scripted() -> Outcome {
    let machine = Machine::new();
    let mut rng = rng::Rng::from_env();
    let mut buf = Vec::new();
    loop {
        let b = match term::read_byte_blocking() {
            Some(b) => b,
            None => return Outcome::Ended,
        };
        if matches_phrase(&mut buf, b, BYPASS) {
            return Outcome::Bypass;
        }
        if is_spin(b) {
            let line = machine.spin(&mut rng);
            println!("{}", ui::plain_line(line));
            if Machine::is_jackpot(line) {
                return Outcome::Won;
            }
        }
    }
}

fn is_spin(key: u8) -> bool {
    matches!(key, b' ' | b'\r' | b'\n')
}

/// Rolling buffer of recent keys; true when it ends with the recovery phrase.
fn matches_phrase(buf: &mut Vec<u8>, key: u8, phrase: &[u8]) -> bool {
    if phrase.is_empty() {
        return false;
    }
    buf.push(key.to_ascii_lowercase());
    if buf.len() > phrase.len() {
        let drop = buf.len() - phrase.len();
        buf.drain(0..drop);
    }
    buf.as_slice() == phrase
}

fn print_usage() {
    println!(
        "slots-boot - spin until you hit 7-7-7, then the machine lets you boot.\n\n\
         Normally launched from the initramfs; there's nothing to configure.\n\
         Set $SLOTS_SEED for a reproducible RNG (used by the tests)."
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spin_keys_are_recognised() {
        assert!(is_spin(b' '));
        assert!(is_spin(b'\n'));
        assert!(is_spin(b'\r'));
        assert!(!is_spin(b'x'));
    }

    #[test]
    fn phrase_matches_only_in_sequence() {
        let phrase = b"letmeboot".to_vec();
        let mut buf = Vec::new();
        for &k in b"xx zzz " {
            assert!(!matches_phrase(&mut buf, k, &phrase));
        }
        let mut hit = false;
        for &k in b"letmeboot" {
            hit = matches_phrase(&mut buf, k, &phrase);
        }
        assert!(hit);
    }

    #[test]
    fn empty_phrase_never_matches() {
        let mut buf = Vec::new();
        assert!(!matches_phrase(&mut buf, b'a', b""));
    }
}
