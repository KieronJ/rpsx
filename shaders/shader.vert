#version 330 core

in vec2 pos;
in vec2 tex;

out vec2 f_tex;

void main()
{
	f_tex = tex;
	gl_Position = vec4(pos, 0.0, 1.0);
}