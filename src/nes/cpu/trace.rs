// use std::collections::VecDeque;
// 
// pub struct Tracer {
//     history: VecDeque<String>,
//     capacity: usize,
// }
// 
// impl Tracer {
//     pub fn new(trace_length: usize) -> Self {
//         Self {
//             history: VecDeque::with_capacity(trace_length),
//             capacity: trace_length,
//         }
//     }
// 
//     pub fn write(&mut self, message: String) {
//         if self.history.len() == self.capacity {
//             self.history.pop_front(); // Remove the oldest entry if at capacity
//         }
//         self.history.push_back(message);
//     }
// 
//     pub fn trace(&self) -> Vec<String> {
//         self.history.iter().cloned().collect()
//     }
// 
//     pub fn print_trace(&self) {
//         for (i, trace_line) in self.history.iter().enumerate() {
//             println!("Trace #{:03}: {trace_line}", i);
//         }
//     }
// }
