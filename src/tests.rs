use super::*;
use std::fs::File;

use tempfile::tempfile;

const ADV_HUCK_FINN: &'static [u8] = include_bytes!("adventures-of-huckleberry-finn.txt");

const SWAP_TEST_PAGE_SZ: usize = 50;
const SWAP_TEST_FRAMES: usize = 50;

const L1_SWAP_TEST_PAGE_SZ: usize = 25;
const L1_SWAP_TEST_FRAMES: usize = 25;
const L2_SWAP_TEST_PAGE_SZ: usize = 100;
const L2_SWAP_TEST_FRAMES: usize = 100;

fn new_test_file() -> File {
    use std::io::Write;
    let mut file = tempfile().expect("Failed to create temp file. This is an OS failure, not a crate bug.");
    let len = file.write(ADV_HUCK_FINN).expect("Failed to write to temp file. This is an OS failure, not a crate bug.");
    if len != ADV_HUCK_FINN.len() {
        panic!("Failed to write to temp file. This is an OS failure, not a crate bug.")
    }
    file
}

fn test_full_cache() -> FullCache<File> {
    FullCache::new(new_test_file()).unwrap()
}

fn test_swap_cache() -> SwapCache<File> {
    SwapCache::new(new_test_file(), SWAP_TEST_PAGE_SZ, SWAP_TEST_FRAMES).unwrap()
}

fn test_auto_cache_swap() -> AutoCache<File> {
    AutoCache::new(new_test_file(), SWAP_TEST_PAGE_SZ * SWAP_TEST_FRAMES).unwrap()
}

fn test_auto_cache_full() -> AutoCache<File> {
    AutoCache::new(new_test_file(), ADV_HUCK_FINN.len()).unwrap()
}

fn test_layered_cache() -> SwapCache<CacheReader<SwapCache<File>>> {
    SwapCache::new(CacheReader::new(SwapCache::new(new_test_file(), L2_SWAP_TEST_PAGE_SZ, L2_SWAP_TEST_FRAMES).unwrap()), L1_SWAP_TEST_PAGE_SZ, L1_SWAP_TEST_FRAMES).unwrap()
}

#[test]
fn full_cache_init_test() {
    let cache = test_full_cache();
    assert_eq!(cache.len(), ADV_HUCK_FINN.len() as u64);
    assert_eq!(cache.cache_size(), ADV_HUCK_FINN.len());
}

#[test]
fn swap_cache_init_test() {
    let cache = test_swap_cache();
    assert_eq!(cache.len(), ADV_HUCK_FINN.len() as u64);
    assert_eq!(cache.cache_size(), SWAP_TEST_PAGE_SZ * SWAP_TEST_FRAMES);
}

#[test]
fn auto_cache_full_init_test() {
    let cache = test_auto_cache_full();
    assert_eq!(cache.len(), ADV_HUCK_FINN.len() as u64);
    assert_eq!(cache.cache_size(), ADV_HUCK_FINN.len());
}

#[test]
fn auto_cache_swap_init_test() {
    let cache = test_auto_cache_swap();
    assert_eq!(cache.len(), ADV_HUCK_FINN.len() as u64);
    assert_eq!(cache.cache_size(), SWAP_TEST_PAGE_SZ * SWAP_TEST_FRAMES);
}

#[test]
fn layered_cache_swap_init_test() {
    let cache = test_layered_cache();
    assert_eq!(cache.len(), ADV_HUCK_FINN.len() as u64);
    assert_eq!(cache.cache_size(), L1_SWAP_TEST_PAGE_SZ * L1_SWAP_TEST_FRAMES);
}

fn general_test_1<C: Cache>(cache: C) {
    let mut buf = vec![0; ADV_HUCK_FINN.len()];

    let count = cache.read(0, &mut buf).unwrap();
    assert_eq!(count, ADV_HUCK_FINN.len());
    assert_eq!(buf, ADV_HUCK_FINN);

    let count = cache.read(0, &mut buf).unwrap();
    assert_eq!(count, ADV_HUCK_FINN.len());
    assert_eq!(buf, ADV_HUCK_FINN);
}

#[test]
fn full_cache_general_test_1() {
    general_test_1(test_full_cache());
}

#[test]
fn swap_cache_general_test_1() {
    general_test_1(test_swap_cache());
}

#[test]
fn auto_cache_full_general_test_1() {
    general_test_1(test_auto_cache_full());
}

#[test]
fn auto_cache_swap_general_test_1() {
    general_test_1(test_auto_cache_swap());
}

#[test]
fn layered_cache_general_test_1() {
    general_test_1(test_layered_cache());
}