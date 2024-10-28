
use std::sync::mpsc;
use std::thread;


// struct Point(i32, i32);


// fn left_most(p1: &Point, p2: &Point) -> &Point {
//     if p1.0 < p2.0 {
//         p1
//     } else {
//         p2
//     }
// }

fn main() {
    // let p1: Point = Point(10, 10);
    // let p2: Point = Point(20, 20);
    // let p3 = left_most(&p1, &p2); // What is the lifetime of p3?
    // println!("p3: {p3:?}");


    // let s: Result<&str,Error> = std::result::Result::Ok("cool");
    // match s {
    //     Ok(s) => println!("cool: {}", s),
    //     Err(err) => println!("not cool {}", err.to_string()),
    // }


    // let (tx, rx) = mpsc::channel();

    // thread::spawn(move || {
    //     let val = String::from("hi");
    //     tx.send(val).unwrap();
    //     println!("val is {val}");
    // });

    // let received = rx.recv().unwrap();
    // println!("Got: {received}");
}