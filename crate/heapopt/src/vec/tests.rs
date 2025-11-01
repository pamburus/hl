// ---

use super::*;

// third-party imports
use more_asserts::*;

#[test]
fn test_vec() {
    let mut vec = Vec::<i32, 3>::new();
    assert_eq!(vec.len(), 0);

    vec.push(1);
    vec.push(2);
    vec.push(3);
    assert_eq!(vec.len(), 3);

    vec.push(4);
    assert_eq!(vec.len(), 4);

    vec.clear();
    assert_eq!(vec.len(), 0);

    vec.push(1);
    vec.push(2);
    vec.push(3);
    vec.truncate(2);
    assert_eq!(vec.len(), 2);

    vec.push(3);
    vec.push(4);
    vec.push(5);
    vec.truncate(4);
    assert_eq!(vec.len(), 4);

    assert_eq!(vec.get(0), Some(&1));
    assert_eq!(vec.get(1), Some(&2));
    assert_eq!(vec.get(3), Some(&4));

    assert_eq!(vec.get_mut(0), Some(&mut 1));
    assert_eq!(vec.get_mut(1), Some(&mut 2));
    assert_eq!(vec.get_mut(3), Some(&mut 4));

    let mut vec = Vec::<i32, 3>::from_slice(&[1, 2, 3, 4]);
    assert_eq!(vec.len(), 4);

    vec.clear();
    vec.extend_from_slice(&[1, 2, 3, 4, 5]);
    assert_eq!(vec.len(), 5);

    assert_eq!(vec.as_slices().0, &[1, 2, 3]);
    assert_eq!(vec.as_slices().1, &[4, 5]);
    assert_eq!(vec.as_mut_slices().0, &mut [1, 2, 3]);
    assert_eq!(vec.as_mut_slices().1, &mut [4, 5]);

    assert_eq!(vec[0], 1);
    assert_eq!(vec[1], 2);
    assert_eq!(vec[2], 3);
    assert_eq!(vec[3], 4);

    vec[1] = 6;
    assert_eq!(vec[1], 6);

    vec[3] = 7;
    assert_eq!(vec[3], 7);

    let mut iter = vec.iter();
    assert_eq!(iter.next(), Some(&1));
    assert_eq!(iter.next(), Some(&6));

    let mut iter = vec.iter_mut();
    assert_eq!(iter.next(), Some(&mut 1));
    assert_eq!(iter.next(), Some(&mut 6));

    assert_eq!(vec.first(), Some(&1));
    assert_eq!(vec.first_mut(), Some(&mut 1));
    assert_eq!(vec.last(), Some(&5));
    assert_eq!(vec.last_mut(), Some(&mut 5));

    assert_eq!(vec.pop(), Some(5));
    assert_eq!(vec.pop(), Some(7));
    assert_eq!(vec.pop(), Some(3));
    assert_eq!(vec.pop(), Some(6));

    assert_eq!(vec.first(), Some(&1));
    assert_eq!(vec.first_mut(), Some(&mut 1));
    assert_eq!(vec.last(), Some(&1));
    assert_eq!(vec.last_mut(), Some(&mut 1));

    assert_eq!(vec.pop(), Some(1));
    assert_eq!(vec.pop(), None);

    assert_eq!(vec.first(), None);
    assert_eq!(vec.first_mut(), None);
    assert_eq!(vec.last(), None);
    assert_eq!(vec.last_mut(), None);

    assert_eq!(Vec::<i32, 2>::with_capacity(3).capacity(), 3);
    assert_eq!(Vec::<i32, 2>::with_capacity(1).capacity(), 2);
    assert_eq!(Vec::<i32, 2>::with_capacity(0).capacity(), 2);

    let mut vec = Vec::<i32, 3>::new();

    vec.reserve(2);
    assert_eq!(vec.capacity(), 3);

    vec.reserve(3);
    assert_eq!(vec.capacity(), 3);

    vec.reserve(4);
    let cap = vec.capacity();
    assert_ge!(cap, 4);

    vec.extend([1, 2].iter().cloned());
    assert_eq!(vec.len(), 2);
    assert_eq!(vec.capacity(), cap);
    assert_eq!(vec.as_slices().0, &[1, 2]);

    vec.extend([3, 4].iter().cloned());
    assert_eq!(vec.len(), 4);
    assert_eq!(vec.capacity(), cap);
    assert_eq!(vec.as_slices().0, &[1, 2, 3]);
    assert_eq!(vec.as_slices().1, &[4]);

    vec.extend([5, 6].iter().cloned());
    assert_eq!(vec.len(), 6);
    assert_ge!(vec.capacity(), 6);
    assert_eq!(vec.as_slices().0, &[1, 2, 3]);
    assert_eq!(vec.as_slices().1, &[4, 5, 6]);
}
