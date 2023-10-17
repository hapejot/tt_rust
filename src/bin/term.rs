//! Demonstrates how to read events asynchronously with tokio.
//!
//! cargo run --features="event-stream" --example event-stream-tokio
#![allow(unused_imports)]

use std::{
    borrow::BorrowMut,
    io::{stdout, Write},
    rc::Rc,
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
    let _ = w.queue(Clear(ClearType::All)).unwrap();
    let mut reader = EventStream::new();
    let pos = Location { x: 10, y: 10 };
    let p = Box::new(Panel::new());
    let mut f = Frame::new(p);
    loop {
        f.write_to(w);
        w.queue(MoveTo(pos.x, pos.y)).unwrap();
        w.flush();
        if let Some(Ok(r)) = reader.next().await {
            goto_cursor_location(w, &pos);
            // w.queue(MoveTo(pos.x, pos.y)).unwrap();
            // w.queue(Print(format!("{:?}", r))).unwrap();
            w.flush().unwrap();
            f.handle_event(r);
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
    // status: TextContent,
    // name: TextContent,
}

impl AppData {
    fn new() -> AppData {
        AppData {
            // status: TextContent::new("status"),
            // name: TextContent::new("name"),
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

    let p = Box::new(Panel::new());
    let f = &Frame::new(p);
    // let mut o = vec![].writer();
    // f.write_to(&mut o);
    // let xo = o.into_inner();
    // assert_eq!(*xo, b"(Panel)(Frame)"[..]);
    let mut o = stdout();
    enable_raw_mode().unwrap();
    o.queue(Clear(ClearType::All)).unwrap();
    f.write_to(&mut o);
    disable_raw_mode().unwrap();
}
