pub fn part_size_for_size(size: u64) -> u64 {
    let min_part_size = 1024 * 1024; // 1 MiB
    let max_parts = 10000;
    let part_size = (size + max_parts - 1) / max_parts; // Round up division
    part_size.next_power_of_two().max(min_part_size)
}

#[cfg(test)]
mod tests {
    use core::num;

    use super::*;

    #[test]
    fn test_part_size_for_size() {
        assert_eq!(part_size_for_size(0), 1024 * 1024);
        assert_eq!(part_size_for_size(1024 * 1024), 1024 * 1024);
        assert_eq!(part_size_for_size(1024 * 1024 + 1), 1024 * 1024);
        assert_eq!(part_size_for_size(1024 * 1024 * 10000), 1024 * 1024 * 10000);
    }

    #[test]
    fn test_part_size_2() {
        let size = 257 * 1024 * 1024 * 1024; // 257 GiB
        let part_size = part_size_for_size(size);
        let num_parts = size / part_size;
        assert!(num_parts <= 10000, "Number of parts {} exceeds limit for size", num_parts);
    }
}