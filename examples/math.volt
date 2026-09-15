module math_demo;

#include <volt/math>

int main() {
    f64 root = sqrt(16.0);
    f64 angle = PI / 2.0;
    f64 one = sin(angle);
    int whole = (root + one) as int;
    return whole;
}
