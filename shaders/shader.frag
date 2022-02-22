#version 330 core

in vec2 f_tex;

out vec4 o_col;

uniform sampler2D t_tex;

void main()
{
	o_col = texture(t_tex, f_tex);
}