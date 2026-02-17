#!/usr/bin/env bash
glslc -fshader-stage=vertex   standard/render_vertex_shader.glsl   -o standard/render_vertex_shader.spv
glslc -fshader-stage=fragment standard/render_fragment_shader.glsl -o standard/render_fragment_shader.spv
glslc -fshader-stage=vertex   standard/display_vertex_shader.glsl   -o standard/display_vertex_shader.spv
glslc -fshader-stage=fragment standard/display_fragment_shader.glsl -o standard/display_fragment_shader.spv
