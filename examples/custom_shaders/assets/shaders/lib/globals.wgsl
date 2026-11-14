// Group 0 belongs to the engine: it binds the camera and the lights for every material draw.
#include "aphyn/standard_types.wgsl"

@group(0) @binding(0) var<uniform> global_data: GlobalData;
@group(0) @binding(1) var<storage, read> lights: array<GenericLight>;
