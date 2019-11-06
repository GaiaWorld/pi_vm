if (typeof print !== 'function') { print = console.log; }

function test() {
    var i;
    var x = 'foo';
    var t;

    for (i = 0; i < 1e6; i++) {
        t = x + 'bar';
        t = x + 'bar';
        t = x + 'bar';
        t = x + 'bar';
        t = x + 'bar';
        t = x + 'bar';
        t = x + 'bar';
        t = x + 'bar';
        t = x + 'bar';
        t = x + 'bar';
    }
    __gc();
}

// try {
//     test();
// } catch (e) {
//     print(e.stack || e);
//     throw e;
// }
