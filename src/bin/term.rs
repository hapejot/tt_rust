//! Demonstrates how to read events asynchronously with tokio.
//!
//! cargo run --features="event-stream" --example event-stream-tokio
#![allow(unused_imports)]

use std::{
    io::{stdout, Write},
    sync::{Arc, Mutex},
    time::Duration,
};

use chrono::Local;

use futures::{future::FutureExt, select, StreamExt};
use futures_timer::Delay;
use tt_rust::{
    controls::{Form, Label, Location, TextContent, TextInput, Widget},
    ui::glyph::{frame::Frame, panel::Panel, Glyph},
};

use crossterm::{
    cursor::{position, MoveTo},
    event::{DisableMouseCapture, EnableMouseCapture, Event, EventStream, KeyCode, KeyEvent},
    execute, queue,
    style::Print,
    terminal::{
        disable_raw_mode, enable_raw_mode, Clear, ClearType, EnterAlternateScreen,
        LeaveAlternateScreen, SetTitle,
    },
    QueueableCommand, Result,
};

/// Prints a rectangular box.
/// # let printer = Printer::new((6,4), &t, &*b);
/// printer.print_box((0, 0), (6, 4), false);
///
pub fn print_box(w: &mut Box<dyn Write>) -> Result<()> {
    w.write("\r\n".as_bytes())?;
    w.write("┌".as_bytes())?;
    w.write("─".as_bytes())?;
    w.write("┐".as_bytes())?;

    w.write("\r\n├─┤".as_bytes())?;

    w.write("\r\n".as_bytes())?;
    w.write("│".as_bytes())?;
    w.write("─".as_bytes())?;
    w.write("│".as_bytes())?;

    w.write("\r\n".as_bytes())?;
    w.write("└".as_bytes())?;
    w.write("─".as_bytes())?;
    w.write("┘".as_bytes())?;

    w.write("\r\n╚═╝".as_bytes())?;
    w.write("\r\n╔═╗".as_bytes())?;
    w.write("\r\n╟─╢".as_bytes())?;
    w.write("\r\n╚═╝".as_bytes())?;
    w.write("\r\n╞═╡".as_bytes())?;
    Ok(())
}

fn goto_cursor_location(w: &mut Box<dyn Write>, l: &Location) {
    let _ = queue!(w, MoveTo(l.x, l.y));
    w.flush().expect("flush");
}

async fn event_loop(w: &mut Box<dyn Write>, _d: &AppData) {
    let _ = w.queue(Clear(ClearType::All)).unwrap().flush();
    let mut reader = EventStream::new();
    let mut term_width = 0;
    let mut term_height = 0;
    let top_left = Location { x: 0, y: 0 };
    let pos = Location { x: 10, y: 10 };
    loop {
        if let Some(Ok(r)) = reader.next().await {
            goto_cursor_location(w, &pos);
            w.queue(Print(format!("{:?}", r))).unwrap();
            w.flush().unwrap();

            match r {
                Event::Key(KeyEvent {
                    code: KeyCode::Esc, ..
                }) => {
                    break;
                }
                Event::Key(_) => todo!(),
                Event::FocusGained => todo!(),
                Event::FocusLost => todo!(),
                Event::Mouse(_) => todo!(),
                Event::Paste(_) => todo!(),
                Event::Resize(width, height) => {
                    term_width = width;
                    term_height = height;
                    w.queue(Clear(ClearType::All)).unwrap();
                    goto_cursor_location(w, &top_left);
                    w.queue(Print("┌".to_string())).unwrap();
                    for _ in 1..(term_width - 1) {
                        w.write("─".as_bytes()).unwrap();
                    }
                    let single_edge_top_left = b"\xE2\x94\x8C";
                    let single_edge_top_right = b"\xE2\x94\x90";
                    w.write(single_edge_top_right).unwrap(); // "┐"

                    goto_cursor_location(w, &Location { x: 0, y: 1 });
                    w.write("│".as_bytes()).unwrap();
                    goto_cursor_location(
                        w,
                        &Location {
                            x: term_width - 1,
                            y: 1,
                        },
                    );
                    w.write("│".as_bytes()).unwrap();

                    goto_cursor_location(w, &Location { x: 0, y: 2 });
                    w.write("├".as_bytes()).unwrap();
                    for _ in 1..(term_width - 1) {
                        w.write("─".as_bytes()).unwrap();
                    }
                    w.write("┤".as_bytes()).unwrap();

                    for i in 3..(term_height - 1) {
                        goto_cursor_location(w, &Location { x: 0, y: i });
                        w.write("│".as_bytes()).unwrap();
                        goto_cursor_location(
                            w,
                            &Location {
                                x: term_width - 1,
                                y: i,
                            },
                        );
                        w.write("│".as_bytes()).unwrap();
                    }

                    goto_cursor_location(
                        w,
                        &Location {
                            x: 0,
                            y: term_width - 1,
                        },
                    );
                    w.write("└".as_bytes()).unwrap();
                    for _ in 1..(term_width - 1) {
                        w.write("─".as_bytes()).unwrap();
                    }
                    w.write("┘".as_bytes()).unwrap();

                    w.flush().unwrap();
                }
            }
        }
    }
}

#[allow(dead_code)]
async fn event_loop2(w: &mut Box<dyn Write>, _d: &AppData) {
    let key_c = Event::Key(KeyCode::Char('c').into());
    let mut reader = EventStream::new();
    let _ = w.queue(Clear(ClearType::All)).expect("clear").flush();
    let label1 = Label::new("[Label]");
    let clock_l = Label::new("[clock]");
    let input = TextInput::new(20);
    let mut form = Form::new();
    let active = input.clone();
    let _clock = clock_l.clone();
    form.add(label1);
    form.add(clock_l);
    form.add(input);

    w.queue(SetTitle("Hello 1")).expect("2");

    w.flush().unwrap();
    let cursor = Location { x: 0, y: 0 };
    loop {
        let mut delay = Delay::new(Duration::from_millis(1_000)).fuse();
        let mut event = reader.next().fuse();
        form.draw(w);
        goto_cursor_location(w, &cursor);

        select! {
            _ = delay => {
                // let mut c = clock.txt.content.lock().unwrap();
                // *c = Local::now().format("[%H:%M:%S]").to_string();
                // drop(c);
            },
            maybe_event = event => {
                match maybe_event {
                    Some(Ok(event)) => {
                        if let Some(unhandled_event) = active.handle_event(event){
                            if let Event::Mouse( _kind ) = unhandled_event {
                                // println!("mouse event: {:?}", _kind);
                            }
                            else {
                                if unhandled_event == key_c {
                                    // println!("Cursor position: {:?}", position());
                                }

                                if unhandled_event == Event::Key(KeyCode::Esc.into()) {
                                    break;
                                }
                            }
                        }
                    },
                    Some(Err(e)) => println!("Error: {:?}", e),
                    None => break,
                }
            }
        };
    }
}

// #[derive(Serialize, Deserialize)]
struct AppData {
    status: TextContent,
    name: TextContent,
}

impl AppData {
    fn new() -> AppData {
        AppData {
            status: TextContent::new("status"),
            name: TextContent::new("name"),
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    enable_raw_mode()?;

    execute!(stdout(), EnableMouseCapture, EnterAlternateScreen)?;
    let w = &mut (Box::new(stdout()) as Box<dyn Write>);
    let data = AppData::new();
    event_loop(w, &data).await;

    execute!(stdout(), LeaveAlternateScreen, DisableMouseCapture)?;

    disable_raw_mode()
}

#[tokio::test]
async fn ui1() {
    use bytes::BufMut;
    use std::io::Write;

    let p = Panel::new();
    let f = &Frame::new(Box::new(p));
    // let mut o = vec![].writer();
    // f.write_to(&mut o);
    // let xo = o.into_inner();
    // assert_eq!(*xo, b"(Panel)(Frame)"[..]);
    let mut o = stdout();
    f.write_to(&mut o);
}
