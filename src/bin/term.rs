//! Demonstrates how to read events asynchronously with tokio.
//!
//! cargo run --features="event-stream" --example event-stream-tokio
#![allow(unused_imports)]

use std::{
    borrow::BorrowMut,
    collections::{vec_deque, VecDeque},
    fmt,
    io::{stdout, Write},
    rc::Rc,
    sync::{Arc, Mutex},
    time::Duration,
};

use chrono::Local;

use futures::{future::FutureExt, select, StreamExt};
use futures_timer::Delay;
use tracing::*;
use tt_rust::ui::glyph::{
    frame::Frame, input::Input, label::Label, panel::Panel, AppRequest, AppResponse, AppResult,
    Glyph,
};

use crossterm::{
    cursor::{position, MoveTo},
    event::{
        DisableMouseCapture, EnableMouseCapture, Event, EventStream, KeyCode, KeyEvent,
        KeyEventKind, KeyModifiers, MouseEvent,
    },
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

async fn event_loop(w: &mut Box<dyn Write>, _d: AppData) -> AppData {
    let mut appdata = _d;

    let _ = w.queue(Clear(ClearType::All)).unwrap();
    let mut reader = EventStream::new();

    let mut f = appdata.form;
    let mut x = 0;
    let mut y = 0;
    let mut requests: VecDeque<AppRequest> = VecDeque::new();
    let mut responses: VecDeque<AppResult> = VecDeque::new();
    for (name, val) in appdata.values.iter() {
        requests.push_back(AppRequest::SetValue {
            name: name.clone(),
            value: val.clone(),
        });
    }

    requests.push_back(AppRequest::NextInput(0, 0));

    loop {
        if let Some(Ok(r)) = reader.next().await {
            match r {
                Event::Key(KeyEvent {
                    code: KeyCode::Esc, ..
                }) => break,
                Event::Key(KeyEvent {
                    code: KeyCode::Up,
                    kind: KeyEventKind::Press,
                    ..
                }) => {
                    if y > 0 {
                        y -= 1;
                    }
                }
                Event::Key(KeyEvent {
                    code: KeyCode::Down,
                    kind: KeyEventKind::Press,
                    ..
                }) => {
                    y += 1;
                }
                Event::Key(KeyEvent {
                    code: KeyCode::Left,
                    kind: KeyEventKind::Press,
                    ..
                }) => {
                    if x > 0 {
                        x -= 1;
                    }
                }
                Event::Key(KeyEvent {
                    code: KeyCode::Right,
                    kind: KeyEventKind::Press,
                    ..
                }) => {
                    x += 1;
                }
                Event::Key(KeyEvent {
                    code: KeyCode::Enter,
                    modifiers: KeyModifiers::CONTROL,
                    kind: KeyEventKind::Press,
                    ..
                }) => {
                    break;
                }
                Event::Key(KeyEvent {
                    code: KeyCode::Enter,
                    modifiers: KeyModifiers::NONE,
                    kind: KeyEventKind::Press,
                    ..
                }) => requests.push_back(AppRequest::NextInput(x, y)),
                Event::Mouse(MouseEvent {
                    kind, column, row, ..
                }) => match kind {
                    crossterm::event::MouseEventKind::Down(_) => {
                        x = column;
                        y = row;
                    }
                    crossterm::event::MouseEventKind::Up(_) => {}
                    crossterm::event::MouseEventKind::Drag(_) => {}
                    crossterm::event::MouseEventKind::Moved => {}
                    crossterm::event::MouseEventKind::ScrollDown => {}
                    crossterm::event::MouseEventKind::ScrollUp => {}
                },
                r => match f.handle_term_event(r) {
                    Ok(c) => {
                        responses.push_back(c);
                    }
                    Err(_) => {}
                },
            }

            while requests.len() > 0 {
                let req = requests.pop_front().unwrap();
                trace!("process Request {:?}", &req);
                if let AppRequest::NextInput(_, _) = req {
                    if !f.allocated() {
                        requests.push_front(req);
                        trace!("request pushed back");
                        break;
                    }
                }
                if let Ok(result) = f.handle_app_request(&req) {
                    responses.push_back(result);
                }
            }

            let mut redraw = false;
            while responses.len() > 0 {
                let result = responses.pop_front().unwrap();
                trace!("process Result: {:?}", result);
                match result {
                    AppResult::StringValue(_) => todo!(),
                    AppResult::Redraw => {
                        w.queue(MoveTo(x, y)).unwrap();
                        w.flush().unwrap();
                        redraw = true;
                    }
                    AppResult::InputEnabled => todo!(),
                    AppResult::NewCursorPosition(new_x, new_y) => {
                        x = new_x;
                        y = new_y;
                        redraw = true;
                    }
                    _ => {}
                }
            }
            if redraw {
                f.write_to(w);
            }
            if let Ok(AppResult::InputEnabled) = f.hit(x, y) {
                // w.queue(crossterm::cursor::Show).unwrap();
                w.queue(crossterm::cursor::SetCursorStyle::BlinkingBlock)
                    .unwrap();
            } else {
                // w.queue(crossterm::cursor::Hide).unwrap();
                w.queue(crossterm::cursor::SetCursorStyle::SteadyBar)
                    .unwrap();
            }
            w.queue(MoveTo(x, y)).unwrap();
            w.flush().unwrap();
        }
    }

    if let Ok(AppResult::Values(vs)) = f.handle_app_request(&AppRequest::CollectAllValues) {
        appdata.values = vs;
    } else {
        error!("no values");
    }
    appdata.form = f;
    appdata
}

#[allow(dead_code)]
async fn event_loop2(_w: &mut Box<dyn Write>, _d: &AppData) {
    // let key_c = Event::Key(KeyCode::Char('c').into());
    // let mut reader = EventStream::new();
    // let _ = w.queue(Clear(ClearType::All)).expect("clear").flush();
    // let label1 = Label::new("[Label]");
    // let clock_l = Label::new("[clock]");
    // let input = TextInput::new(20);
    // let mut form = Form::new();
    // let active = input.clone();
    // let _clock = clock_l.clone();
    // form.add(label1);
    // form.add(clock_l);
    // form.add(input);

    // w.queue(SetTitle("Hello 1")).expect("2");

    // w.flush().unwrap();
    // let cursor = Location { x: 0, y: 0 };
    // loop {
    //     let mut delay = Delay::new(Duration::from_millis(1_000)).fuse();
    //     let mut event = reader.next().fuse();
    //     form.draw(w);
    //     goto_cursor_location(w, &cursor);

    //     select! {
    //         _ = delay => {
    //             // let mut c = clock.txt.content.lock().unwrap();
    //             // *c = Local::now().format("[%H:%M:%S]").to_string();
    //             // drop(c);
    //         },
    //         maybe_event = event => {
    //             match maybe_event {
    //                 Some(Ok(event)) => {
    //                     if let Some(unhandled_event) = active.handle_event(event){
    //                         if let Event::Mouse( _kind ) = unhandled_event {
    //                             // println!("mouse event: {:?}", _kind);
    //                         }
    //                         else {
    //                             if unhandled_event == key_c {
    //                                 // println!("Cursor position: {:?}", position());
    //                             }

    //                             if unhandled_event == Event::Key(KeyCode::Esc.into()) {
    //                                 break;
    //                             }
    //                         }
    //                     }
    //                 },
    //                 Some(Err(e)) => println!("Error: {:?}", e),
    //                 None => break,
    //             }
    //         }
    //     };
    // }
}

// #[derive(Serialize, Deserialize)]
struct AppData {
    // status: TextContent,
    // name: TextContent,
    form: Box<dyn Glyph>,
    values: Vec<(String, String)>,
}

impl AppData {
    fn new(form: Box<dyn Glyph>) -> Self {
        Self {
            form,
            values: vec![], // status: TextContent::new("status"),
                            // name: TextContent::new("name"),
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    use tracing_subscriber::filter::LevelFilter;
    let log_file = std::fs::File::create("term.log")?;
    let subscriber = tracing_subscriber::fmt()
        .with_writer(Mutex::new(log_file))
        .with_ansi(false)
        .with_max_level(LevelFilter::TRACE)
        .finish();
    tracing::subscriber::set_global_default(subscriber).unwrap();
    info!("starting");

    enable_raw_mode()?;

    execute!(stdout(), EnableMouseCapture, EnterAlternateScreen)?;
    let w = &mut (Box::new(stdout()) as Box<dyn Write>);

    let mut p = Box::new(Panel::new());
    let label = Label::new("1".to_string(), "Formular".to_string());
    p.add(Box::new(label));
    p.add(Box::new(Label::new("2".to_string(), "Command".to_string())));
    p.add(Box::new(Input::new("cmd".to_string(), String::new())));
    p.add(Box::new(Label::new("3".to_string(), "Value 1".to_string())));
    p.add(Box::new(Input::new("v1".to_string(), String::new())));
    p.add(Box::new(Label::new("4".to_string(), "Value 2".to_string())));
    p.add(Box::new(Input::new("v2".to_string(), String::new())));
    p.add(Box::new(Label::new("5".to_string(), "Value 3".to_string())));
    p.add(Box::new(Input::new("v3".to_string(), String::new())));
    let mut data = AppData::new(Box::new(Frame::new(p)));

    data.values = vec![("cmd".to_string(), "Peter".to_string())];

    data = event_loop(w, data).await;

    info!("result: {:#?}", data.values);

    execute!(stdout(), LeaveAlternateScreen, DisableMouseCapture)?;

    disable_raw_mode()
}


// #[tokio::test]
// async fn ui1() {
//     use bytes::BufMut;
//     use std::io::Write;

//     let p = Box::new(Panel::new());
//     let f = &Frame::new(p);
//     // let mut o = vec![].writer();
//     // f.write_to(&mut o);
//     // let xo = o.into_inner();
//     // assert_eq!(*xo, b"(Panel)(Frame)"[..]);
//     let mut o = stdout();
//     enable_raw_mode().unwrap();
//     o.queue(Clear(ClearType::All)).unwrap();
//     f.write_to(&mut o);
//     disable_raw_mode().unwrap();
// }
