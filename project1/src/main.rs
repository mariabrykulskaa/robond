fn main() {
    println!("Project 1!");
}

#[allow(dead_code)]
fn add(a: i32, b: i32) -> i32 {
    a + b
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_simple() {
        assert_eq!(add(2, 2), 4);
    }

    #[test]
    fn test_add_with_negative_number() {
        assert_eq!(add(5, -2), 3);
    }
}
