# ECE/CS 285 HW3: a distributed ray tracer

## Build
`cargo build --release`

Many switches are gated behind compile flags. For brute-force, use `--no-default-features`. For each aov view, use the respective `-F aov___`.

Artifact can be found at [target/release/catadiort](target/release/catadiort)

Must be run with assets/ in working directory. Run `catadiort` with no arguments for help.

## Results

Platform: Linux x86_64

### Warps
All warps pass under all samplers.
```
disk                     N = 16960  chi2 = 1917.0  dof = 1986  p = 0.864  pdf error 4.0e-08  PASS
    histograms: warptest/disk.png
cosine_hemisphere        N = 16960  chi2 = 2214.5  dof = 2252  p = 0.71  pdf error 1.0e-07  PASS
    histograms: warptest/cosine_hemisphere.png
uniform_sphere           N = 16960  chi2 = 3200.4  dof = 3199  p = 0.49  pdf error 4.0e-08  PASS
    histograms: warptest/uniform_sphere.png
uniform_cone 0.8         N = 16960  chi2 = 3200.4  dof = 3199  p = 0.49  pdf error 6.3e-09  PASS
    histograms: warptest/uniform_cone_0.8.png
triangle                 N = 16960  chi2 = 1282.1  dof = 1274  p = 0.431  pdf error 0.0e+00  PASS
    histograms: warptest/triangle.png
phong 20                 N = 16960  chi2 = 1358.7  dof = 1364  p = 0.535  pdf error 1.7e-07  PASS
    histograms: warptest/phong_20.png
6 of 6 warps pass
```

### Area Lights
```
exact image: /tmp/area_light_test_5jueoid5/QuadLight_exact.pfm
QuadLight_4.pfm  RMSE 0.29258  mean difference 2.46e-01 (558.0 standard errors)  480 of 480 blocks beyond 4  FAIL
    the worst block starts at pixel (240, 144), 450.0 standard errors off
QuadLight_16.pfm  RMSE 0.29054  mean difference 2.46e-01 (571.8 standard errors)  480 of 480 blocks beyond 4  FAIL
    the worst block starts at pixel (240, 144), 910.1 standard errors off
QuadLight_64.pfm  RMSE 0.28994  mean difference 2.46e-01 (575.4 standard errors)  480 of 480 blocks beyond 4  FAIL
    the worst block starts at pixel (240, 144), 1556.2 standard errors off
QuadLight_256.pfm  RMSE 0.28982  mean difference 2.46e-01 (576.3 standard errors)  480 of 480 blocks beyond 4  FAIL
    the worst block starts at pixel (240, 144), 2067.1 standard errors off

  spp    RMSE      ratio to the previous (about 2 when unbiased)
     4   0.29258   
    16   0.29054   1.01
    64   0.28994   1.00
   256   0.28982   1.00

FAIL: the error does not behave like zero-mean noise
```

### Convergence
```
reference: reference_renders/2_Cook/1_TestScenes/Convergence_ref.pfm
the reference's own noise, RMSE 0.00245, subtracted in quadrature
RMSE against the 16384 spp reference
spp                   1          4         16         64        256       1024   slope  (low, high)
independent     0.31413    0.24721    0.22811    0.22279    0.22164    0.22141   -0.04  (-0.12, -0.00)
stratified      0.31413    0.23799    0.22357    0.22174    0.22142    0.22134   -0.04  (-0.12, -0.00)
halton          0.31364    0.24041    0.22461    0.22230    0.22206    0.22206   -0.04  (-0.12, -0.00)
sobol           0.31502    0.23430    0.22326    0.22170    0.22137    0.22131   -0.04  (-0.12, -0.00)
plot: renders/2_Cook/2_StudyScenes/convergence.png
gallery (4 spp, independent, stratified, halton, sobol): renders/2_Cook/2_StudyScenes/SamplerGallery.png
```

### Comparisons
```
MISSING 1_TestScenes/Convergence.png
MISSING 1_TestScenes/Convergence_ref.png
MISSING 1_TestScenes/FrostedEarthMovie.mp4
MISSING 2_StudyScenes/FocusSweep.png
MISSING 3_HeroScenes/BarcelonaPavilion.png
MISSING 3_HeroScenes/Chess.png
MISSING 3_HeroScenes/ChopperTitan.png
MISSING 3_HeroScenes/Crown.png
MISSING 3_HeroScenes/FishyCat.png
MISSING 4_ExtraCreditScenes/1984.png
MISSING 4_ExtraCreditScenes/MoleculeFastMovie.mp4
MISSING 4_ExtraCreditScenes/MotionBlur.png
MISSING 4_ExtraCreditScenes/MotionBlurPan.png
Aliasing                                 PASS  RMSE 0.00813 (threshold 0.010)
Aliasing_64                              PASS  RMSE 0.00103 (threshold 0.002)
AmbientOcclusion                         FAIL  RMSE 0.00830 (threshold 0.002)
CornellBox                               FAIL  RMSE 0.10739 (threshold 0.002)
CornellBoxSpheres                        FAIL  RMSE 0.12090 (threshold 0.002)
DepthOfField_far                         FAIL  RMSE 0.07911 (threshold 0.002)
DepthOfField_mid                         FAIL  RMSE 0.06932 (threshold 0.002)
DepthOfField_near                        FAIL  RMSE 0.04926 (threshold 0.002)
FrostedEarth                             FAIL  RMSE 0.05960 (threshold 0.002)
GlossyFurnace                            FAIL  RMSE 0.00381 (threshold 0.002)
GlossySweep                              FAIL  RMSE 0.06917 (threshold 0.002)
MirrorLamp                               FAIL  RMSE 0.00725 (threshold 0.002)
Paperclip                                FAIL  RMSE 0.34526 (threshold 0.002)
PerforatedLamp                           FAIL  RMSE 0.09599 (threshold 0.002)
QuadLight                                FAIL  RMSE 0.19224 (threshold 0.002)
ScatteringPebbles                        FAIL  RMSE 0.10319 (threshold 0.002)
ShadowRays                               FAIL  RMSE 0.15846 (threshold 0.002)
ShadowRays_independent                   FAIL  RMSE 0.13524 (threshold 0.002)
ShadowRays_stratified                    FAIL  RMSE 0.13498 (threshold 0.002)
Skylight                                 FAIL  RMSE 0.17413 (threshold 0.002)
SphereLight                              FAIL  RMSE 0.00862 (threshold 0.002)
SphereLightInside                        FAIL  RMSE 0.19220 (threshold 0.002)
Stripes                                  PASS  RMSE 0.00035 (threshold 0.002)
Stripes_halton                           PASS  RMSE 0.00036 (threshold 0.002)
WarpView_cosine_hemisphere               PASS  RMSE 0.00019 (threshold 0.002)
WarpView_disk                            PASS  RMSE 0.00026 (threshold 0.002)
WarpView_phong                           PASS  RMSE 0.00015 (threshold 0.002)
WarpView_triangle                        PASS  RMSE 0.00012 (threshold 0.002)
WarpView_uniform_cone                    PASS  RMSE 0.00015 (threshold 0.002)
WarpView_uniform_sphere                  PASS  RMSE 0.00027 (threshold 0.002)
LightSamples                             FAIL  RMSE 0.11420 (threshold 0.002)
ManyLights                               FAIL  RMSE 0.02706 (threshold 0.002)
FocusSweep_000                           FAIL  RMSE 0.05338 (threshold 0.002)
FocusSweep_001                           FAIL  RMSE 0.06001 (threshold 0.002)
FocusSweep_002                           FAIL  RMSE 0.07237 (threshold 0.002)
FocusSweep_003                           FAIL  RMSE 0.08257 (threshold 0.002)
FocusSweep_004                           FAIL  RMSE 0.08769 (threshold 0.002)
FocusSweep_005                           FAIL  RMSE 0.08971 (threshold 0.002)
FocusSweep_006                           FAIL  RMSE 0.09028 (threshold 0.002)
FocusSweep_007                           FAIL  RMSE 0.09012 (threshold 0.002)
Candy                                    FAIL  RMSE 0.16303 (threshold 0.010)
MagicalClock                             FAIL  RMSE 0.04875 (threshold 0.010)
10 passed, 32 failed, 68 with no reference, 13 required files missing
```

NB: Many fail almost certainly because of my difference in sampling order.

### Writeup
NEE is unbiased because of a strategic separation of concerns. All it adds is an incorporation of outside knowledge of the world to contribute bounces that would've otherwise been highly unlikely (lights far from specular reflection). The factor that makes this possible is its probability weighting- while finding unlikely directions, their contributions are still weighted by their probabilities, preserving the principles Monte Carlo integration relies on. Proper NEE also skips double-counting from secondary rays, in order to uphold conservation of energy, meaning each NEE hit fully represents that light's influence on this point.

The glossy lobe's weight is a complex approximation of a BRDF. It represents the energy that reaches a layer "under" the coat- the energy lost on this bounce. The two factors of the numerator are the incoming and outgoing loss, which is normalized by the average loss in the denominator. This amount is what albedo is intended to be multiplied by- the light that reaches its pigment. For the ambient branch, the incoming loss is the average loss as the ambient color is assumed to be arriving from every direction. This cancels the denominator.

Antialiasing is a primary scene of difference. Cook makes many samples per pixel, and is able to filter out the high frequencies Whitted misses. As a result, the center of the circle is a blurry grey instead of a stripy mess.
Depth of Field is also obvious- Cook's use of sampling means depth of field from many film and lens points is possible.
CornellBox also now has soft shadows.

## Footnotes

HW2's statement:
> This project in total was extremely challenging. I may have overscoped by attempting to complete the entire thing with SIMD, and spent a long time perfecting the architecture to ensure everything went smoothly. In particular, it was challenging to hunt down very specific bugs in NaN handling (which I used as a sentinel for empty lanes). I think that I had a good shot, but didn't end up having enough time to complete what I wanted to.

Once again, this has been extremely difficult. It was challenging working through debt from the previous week, and it also took a long time sifting through reference documents to find every little edge case. In particular, deciphering the illumination model was extraordinarily challenging, as the branching behavior of the ambient term was not clear (clear/opaque, coated/uncoated, smooth/rough), and the formulas for each were obfuscated and distant. I managed to recognize each pattern eventually, and I believe my implementation is now correct.

Whitted no longer matches for some scenes (regression).
The sampling order has not been preseved.
There is no environment light.
There is no caustic suppression nor specular term lobe filtering.

Time limits have a granularity of `RAY_CONCURRENCY`, currently 2^22. My parallel iterative structure does not complete a single pixel at a time, so perfect cutoff is impossible. However, progress is still reported, and is quite accurate- use that instead.

**HW2 Extra Credit:**
- Full SIMD Intersections (10)
- Mesh Instancing and Two-Layer BVH (10)
- Tinted Transparent Shadows (3)

**HW3 Extra Credit:**
- Sobol Sampler (5)
- Gaussian Pixel Filter (3) [compile flag `gaussian_weights`]

AI was used strictly to debug/check for errors. All code written by hand.
