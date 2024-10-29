#version 330 core

uniform vec2 u_resolution;
uniform vec3 u_sun_dir;
uniform mat4 u_model_matrix;
uniform mat4 u_view_matrix;
uniform mat4 u_proj_matrix;
uniform mat4 light_mvp; // For shadow mapping

layout (location = 0) in vec3 Position;
layout (location = 1) in vec3 Normal_modelspace;
layout (location = 2) in vec3 texture_coord;
layout (location = 3) in vec3 Color;

out vec3 texCoord;
out vec3 color;
out vec3 Normal_cameraspace;
out vec3 LightDirection_cameraspace;
out vec4 light_space_pos; // For shadow mapping

void main()
{
    vec4 uv = u_proj_matrix * u_view_matrix * u_model_matrix * vec4(Position, 1.0);

    if (u_resolution.x > u_resolution.y) {
        uv.x *= u_resolution.y / u_resolution.x;
    } else {
        uv.y *= u_resolution.x / u_resolution.y;
    }

    // Vertex normal, converted to camera space
	Normal_cameraspace = Normal_modelspace;

    // Vector from vector to eye in camera space
	LightDirection_cameraspace = u_sun_dir;

    gl_Position = uv;
    texCoord = texture_coord;
    color = Color;
    light_space_pos = light_mvp * u_model_matrix * vec4(Position, 1.0); // For shadow mapping
}