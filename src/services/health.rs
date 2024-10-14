use log::info;

pub(crate) fn health() -> bool {
    info!("Health check !");
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_health() {
        assert!(health());
    }
}