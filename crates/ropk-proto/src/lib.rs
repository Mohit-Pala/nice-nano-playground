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
