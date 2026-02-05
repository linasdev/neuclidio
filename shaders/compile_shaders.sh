#!/usr/bin/env bash
glslc -fshader-stage=vertex   standard/vertex_shader.glsl   -o standard/vertex_shader.spv
glslc -fshader-stage=fragment standard/fragment_shader.glsl -o standard/fragment_shader.spv
