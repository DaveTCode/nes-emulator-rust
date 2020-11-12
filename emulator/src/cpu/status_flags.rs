bitflags! {
  #[derive(Default)]
  pub(crate) struct StatusFlags: u8 {
    const CARRY_FLAG             = 0b0000_0001;
    const ZERO_FLAG              = 0b0000_0010;
    const INTERRUPT_DISABLE_FLAG = 0b0000_0100;
    const DECIMAL_FLAG           = 0b0000_1000;
    const OVERFLOW_FLAG          = 0b0100_0000;
    const NEGATIVE_FLAG          = 0b1000_0000;
  }
}

#[cfg(test)]
mod status_flag_tests {
    use super::StatusFlags;

    #[test]
    fn test_empty_status() {
        let f = StatusFlags::empty();
        assert_eq!(f.is_empty(), true);
        assert_eq!("(empty)", format!("{:?}", f));
    }

    #[test]
    fn test_all_set() {
        let f = StatusFlags::CARRY_FLAG
            | StatusFlags::ZERO_FLAG
            | StatusFlags::DECIMAL_FLAG
            | StatusFlags::INTERRUPT_DISABLE_FLAG
            | StatusFlags::NEGATIVE_FLAG
            | StatusFlags::OVERFLOW_FLAG;
        assert_ne!(f.is_empty(), true);
        assert_eq!(
            "CARRY_FLAG | ZERO_FLAG | INTERRUPT_DISABLE_FLAG | DECIMAL_FLAG | OVERFLOW_FLAG | NEGATIVE_FLAG",
            format!("{:?}", f)
        )
    }
}
