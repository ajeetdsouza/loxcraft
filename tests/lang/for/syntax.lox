// Single-expression body.
//
// out: 1
// out: 2
// out: 3
//
for (var c = 0; c < 3;) print c = c + 1;

// Block body.
//
// out: 0
// out: 1
// out: 2
//
for (var a = 0; a < 3; a = a + 1) {
  print a;
}

// No clauses.
//
// out: done
//
fun foo() {
  for (;;) return "done";
}
print foo();

// No variable.
//
// out: 0
// out: 1
//
var i = 0;
for (; i < 2; i = i + 1) print i;

// No condition.
//
// out: 0
// out: 1
// out: 2
//
fun bar() {
  for (var i = 0;; i = i + 1) {
    print i;
    if (i >= 2) return;
  }
}
bar();


// No increment.
//
// out: 0
// out: 1
//
for (var i = 0; i < 2;) {
  print i;
  i = i + 1;
}


// Statement bodies.
for (; false;) if (true) 1; else 2;
for (; false;) while (true) 1;
for (; false;) for (;;) 1;
