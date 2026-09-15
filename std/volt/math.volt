module volt.math;

/// C++ `<cmath>` / `<math.h>` — doubles use the C++ names, floats use the `f` suffix.
#include <math.h>
#include <stdlib.h>

static f64 PI = 3.141592653589793;
static f64 E = 2.718281828459045;
static f64 SQRT2 = 1.4142135623730951;

extern int abs(int x);
extern f64 fabs(f64 x);
extern f32 fabsf(f32 x);

extern f64 sin(f64 x);
extern f64 cos(f64 x);
extern f64 tan(f64 x);
extern f64 asin(f64 x);
extern f64 acos(f64 x);
extern f64 atan(f64 x);
extern f64 atan2(f64 y, f64 x);
extern f32 sinf(f32 x);
extern f32 cosf(f32 x);
extern f32 tanf(f32 x);
extern f32 asinf(f32 x);
extern f32 acosf(f32 x);
extern f32 atanf(f32 x);
extern f32 atan2f(f32 y, f32 x);

extern f64 sinh(f64 x);
extern f64 cosh(f64 x);
extern f64 tanh(f64 x);
extern f64 asinh(f64 x);
extern f64 acosh(f64 x);
extern f64 atanh(f64 x);
extern f32 sinhf(f32 x);
extern f32 coshf(f32 x);
extern f32 tanhf(f32 x);
extern f32 asinhf(f32 x);
extern f32 acoshf(f32 x);
extern f32 atanhf(f32 x);

extern f64 exp(f64 x);
extern f64 exp2(f64 x);
extern f64 expm1(f64 x);
extern f64 log(f64 x);
extern f64 log10(f64 x);
extern f64 log2(f64 x);
extern f64 log1p(f64 x);
extern f32 expf(f32 x);
extern f32 exp2f(f32 x);
extern f32 expm1f(f32 x);
extern f32 logf(f32 x);
extern f32 log10f(f32 x);
extern f32 log2f(f32 x);
extern f32 log1pf(f32 x);

extern f64 pow(f64 base, f64 exp);
extern f64 sqrt(f64 x);
extern f64 cbrt(f64 x);
extern f64 hypot(f64 x, f64 y);
extern f64 fma(f64 x, f64 y, f64 z);
extern f32 powf(f32 base, f32 exp);
extern f32 sqrtf(f32 x);
extern f32 cbrtf(f32 x);
extern f32 hypotf(f32 x, f32 y);
extern f32 fmaf(f32 x, f32 y, f32 z);

extern f64 ceil(f64 x);
extern f64 floor(f64 x);
extern f64 trunc(f64 x);
extern f64 round(f64 x);
extern f64 nearbyint(f64 x);
extern f64 rint(f64 x);
extern f32 ceilf(f32 x);
extern f32 floorf(f32 x);
extern f32 truncf(f32 x);
extern f32 roundf(f32 x);
extern f32 nearbyintf(f32 x);
extern f32 rintf(f32 x);

extern f64 fmod(f64 x, f64 y);
extern f64 remainder(f64 x, f64 y);
extern f64 fmax(f64 x, f64 y);
extern f64 fmin(f64 x, f64 y);
extern f64 fdim(f64 x, f64 y);
extern f64 copysign(f64 mag, f64 sgn);
extern f32 fmodf(f32 x, f32 y);
extern f32 remainderf(f32 x, f32 y);
extern f32 fmaxf(f32 x, f32 y);
extern f32 fminf(f32 x, f32 y);
extern f32 fdimf(f32 x, f32 y);
extern f32 copysignf(f32 mag, f32 sgn);

extern f64 ldexp(f64 x, int exp);
extern f64 scalbn(f64 x, int n);
extern f32 ldexpf(f32 x, int exp);
extern f32 scalbnf(f32 x, int n);
