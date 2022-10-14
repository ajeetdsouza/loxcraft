use lox_vm::VM;

fn main() {
    let mut stdout = std::io::stdout().lock();
    VM::default()
        .run(
            r#"
            var qwerty = clock();

            // Draw a nice Mandelbrot set!

            var xmin = -2.0;
            var xmax = 1.0;
            var ymin = -1.5;
            var ymax = 1.5;
            var width = 1280.0;
            var height = 720.0;
            var threshhold = 1000;

            fun in_mandelbrot(x0, y0, n) {
                var x = 0.0;
                var y = 0.0;
                var xtemp;
                while (n > 0) {
                    xtemp = x*x - y*y + x0;
                    y = 2.0*x*y + y0;
                    x = xtemp;
                    n = n - 1;
                    if (x*x + y*y > 4.0) {
                        return false;
                    }
                }
                return true;
            }

            fun mandel() {
                 var dx = (xmax - xmin)/width;
                 var dy = (ymax - ymin)/height;

                 var y = ymax;
                 var x;

                 while (y >= ymin) {
                     x = xmin;
                 var line = "";
                     while (x < xmax) {
                         if (in_mandelbrot(x, y, threshhold)) {
                            line = line + "*";
                         } else {
                            line = line + ".";
                         }
                         x = x + dx;
                     }
                     print line;
                     y = y - dy;

                 }
            }

            mandel();

            print clock() - qwerty;
    "#,
            &mut stdout,
        )
        .unwrap();
}
