use test;
use uuid::Uuid;

#[bench]
fn uuid_generation(b: &mut test::Bencher) {
    b.iter(|| {
        let x = Uuid::new_v4();
    });
}
