use deep_learning_rs::datasets::mnist::MNIST;

use std::env;

// Save an image from the MNIST dataset.

fn main() {
   let args: Vec<String> = env::args().collect();
   let image_num = if args.len() > 1 {
      args[1].parse::<usize>().unwrap()
   } else {
      0
   };

   let root = "data";
   let mnist = MNIST::new(&root, true).unwrap();
   let image = &mnist.train_images[image_num].to_image();
   let label = mnist.train_labels[image_num];
   image
      .save(format!("mnist_image_{}_{}.png", image_num, label))
      .unwrap();
}
