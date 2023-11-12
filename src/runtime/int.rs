use super::{Receiver, str::StringReceiver};

pub struct IntReceiver(pub isize);

impl Receiver for IntReceiver {
    fn receive_message(
        &self,
        selector: &'static str,
        args: &[&dyn Receiver],
    ) -> Box<dyn Receiver> {
        match selector {
            "+" => Box::new(IntReceiver(self.0 + args[0].as_int().unwrap())),
            "*" => Box::new(IntReceiver(self.0 * args[0].as_int().unwrap())),
            "basic_write_to" => {
                let a0 = StringReceiver(format!("{}", self.0));
                args[0].receive_message("write", &[&a0])
            },
            _ => todo!("message '{}' for int", selector),
        }
    }

    fn as_int(&self) -> Option<isize> {
        Some(self.0)
    }
    fn as_str(&self) -> Option<&'static str> {
        None
    }

}
