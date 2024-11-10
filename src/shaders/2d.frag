#version 330 core

uniform float u_opacity;
uniform sampler2D texture0;
uniform vec2 u_texture_top_left;
uniform vec2 u_texture_size;

in vec3 uv;

out vec4 Color;

void main()
{
    vec4 texture_color = texture(texture0, u_texture_top_left + uv.xy * u_texture_size);
    float texture_alpha = texture_color.w;

    Color = vec4(texture_color.xyz, 1.0);
}