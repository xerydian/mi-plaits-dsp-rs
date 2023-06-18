//! FM data lookup table.

// Based on MIT-licensed code (c) 2016 by Emilie Gillet (emilie.o.gillet@gmail.com)

#![allow(clippy::excessive_precision)]

pub const LUT_FM_FREQUENCY_QUANTIZER: [f32; 130] = [
    -1.200000000e+01,
    -1.200000000e+01,
    -1.200000000e+01,
    -1.184000000e+01,
    -1.184000000e+01,
    -1.184000000e+01,
    -1.111000000e+01,
    -1.038000000e+01,
    -9.650000000e+00,
    -8.920000000e+00,
    -8.190000000e+00,
    -7.460000000e+00,
    -6.730000000e+00,
    -6.000000000e+00,
    -6.000000000e+00,
    -6.000000000e+00,
    -5.545511612e+00,
    -5.091023223e+00,
    -4.636534835e+00,
    -4.182046446e+00,
    -4.182046446e+00,
    -4.182046446e+00,
    -3.659290641e+00,
    -3.136534835e+00,
    -2.613779029e+00,
    -2.091023223e+00,
    -1.568267417e+00,
    -1.045511612e+00,
    -5.227558058e-01,
    0.000000000e+00,
    0.000000000e+00,
    0.000000000e+00,
    1.600000000e-01,
    1.600000000e-01,
    1.600000000e-01,
    8.900000000e-01,
    1.620000000e+00,
    2.350000000e+00,
    3.080000000e+00,
    3.810000000e+00,
    4.540000000e+00,
    5.270000000e+00,
    6.000000000e+00,
    6.000000000e+00,
    6.000000000e+00,
    6.454488388e+00,
    6.908976777e+00,
    7.363465165e+00,
    7.817953554e+00,
    7.817953554e+00,
    7.817953554e+00,
    8.285529931e+00,
    8.753106309e+00,
    9.220682687e+00,
    9.688259065e+00,
    9.688259065e+00,
    9.688259065e+00,
    1.026619430e+01,
    1.084412953e+01,
    1.142206477e+01,
    1.200000000e+01,
    1.200000000e+01,
    1.200000000e+01,
    1.216000000e+01,
    1.216000000e+01,
    1.216000000e+01,
    1.262977500e+01,
    1.309955001e+01,
    1.356932501e+01,
    1.403910002e+01,
    1.403910002e+01,
    1.403910002e+01,
    1.490761987e+01,
    1.577613972e+01,
    1.664465957e+01,
    1.751317942e+01,
    1.751317942e+01,
    1.751317942e+01,
    1.800000000e+01,
    1.800000000e+01,
    1.800000000e+01,
    1.850977500e+01,
    1.901955001e+01,
    1.901955001e+01,
    1.901955001e+01,
    1.981795355e+01,
    1.981795355e+01,
    1.981795355e+01,
    2.066386428e+01,
    2.150977500e+01,
    2.150977500e+01,
    2.150977500e+01,
    2.213233125e+01,
    2.275488750e+01,
    2.337744375e+01,
    2.400000000e+01,
    2.400000000e+01,
    2.400000000e+01,
    2.450977500e+01,
    2.501955001e+01,
    2.501955001e+01,
    2.501955001e+01,
    2.547403840e+01,
    2.592852679e+01,
    2.638301517e+01,
    2.683750356e+01,
    2.683750356e+01,
    2.683750356e+01,
    2.735032035e+01,
    2.786313714e+01,
    2.786313714e+01,
    2.786313714e+01,
    2.839735285e+01,
    2.893156857e+01,
    2.946578428e+01,
    3.000000000e+01,
    3.000000000e+01,
    3.000000000e+01,
    3.075000000e+01,
    3.150000000e+01,
    3.225000000e+01,
    3.300000000e+01,
    3.375000000e+01,
    3.450000000e+01,
    3.525000000e+01,
    3.600000000e+01,
    3.600000000e+01,
    3.600000000e+01,
    3.600000000e+01,
    3.600000000e+01,
];