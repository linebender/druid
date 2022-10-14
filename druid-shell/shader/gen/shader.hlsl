static const uint3 gl_WorkGroupSize = uint3(16u, 16u, 1u);

RWByteAddressBuffer _24 : register(u0);
RWTexture2D<unorm float4> image : register(u1);

static uint3 gl_GlobalInvocationID;
struct SPIRV_Cross_Input
{
    uint3 gl_GlobalInvocationID : SV_DispatchThreadID;
};

void comp_main()
{
    uint2 xy = gl_GlobalInvocationID.xy;
    float2 fragCoord = (float2(gl_GlobalInvocationID.xy) / float2(float(_24.Load(0)), float(_24.Load(4)))) - 0.5f.xx;
    float4 fragColor = float4(fragCoord.x + 0.5f, fragCoord.y + 0.5f, 0.5f + (0.5f * sin(asfloat(_24.Load(8)))), 1.0f);
    image[int2(xy)] = fragColor;
}

[numthreads(16, 16, 1)]
void main(SPIRV_Cross_Input stage_input)
{
    gl_GlobalInvocationID = stage_input.gl_GlobalInvocationID;
    comp_main();
}
