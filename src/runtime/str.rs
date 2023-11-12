use super::{Receiver, sel::SelectorSet};

pub struct StringReceiver(pub String);

impl Receiver for StringReceiver {
    fn receive_message(
        &self,
        selector: &'static str,
        _args: &[&dyn Receiver],
    ) -> Box<dyn Receiver> {
        todo!("implement {} for str", selector)
    }

    fn as_int(&self) -> Option<isize> {
        match str::parse(self.0.as_str()) {
            Ok(i) => Some(i),
            Err(_) => None,
        }
    }
    fn as_str(&self) -> Option<&'static str> {
        Some(SelectorSet::get(self.0.as_str()))
    }
}
