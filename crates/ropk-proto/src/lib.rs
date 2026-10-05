pub mod bond; //james 
pub mod bytes;
pub mod sc_controller_buttons;

pub fn placeholder() -> u32 {
    1
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_builds() {
        assert_eq!(placeholder(), 1);
    }
}
