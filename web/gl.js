// The WebGL 2 renderer. It uploads what the core returns and draws it; every
// size, stride and offset comes from the core's stage_numbers(), so this file
// holds no layout of its own. Raiders of the Lost Mart's renderer (which is
// Ribbit King's and Long Time Comin's) without its facade and see-through
// passes: solid, ink (front faces culled, which leaves the outline) and glow
// (added).

function compile(gl, type, source) {
  const shader = gl.createShader(type);
  gl.shaderSource(shader, source);
  gl.compileShader(shader);
  if (!gl.getShaderParameter(shader, gl.COMPILE_STATUS)) {
    throw new Error(gl.getShaderInfoLog(shader));
  }
  return shader;
}

function link(gl, vertexSource, fragmentSource) {
  const program = gl.createProgram();
  gl.attachShader(program, compile(gl, gl.VERTEX_SHADER, vertexSource));
  gl.attachShader(program, compile(gl, gl.FRAGMENT_SHADER, fragmentSource));
  gl.linkProgram(program);
  if (!gl.getProgramParameter(program, gl.LINK_STATUS)) {
    throw new Error(gl.getProgramInfoLog(program));
  }
  return program;
}

// Returns null when the browser has no WebGL 2; the fight then plays on the
// tiles alone.
export function makeRenderer(canvas, N, shaders, meshData) {
  const gl = canvas.getContext("webgl2", { antialias: true });
  if (!gl) return null;
  const program = link(gl, shaders.vertex, shaders.fragment);
  gl.useProgram(program);
  gl.enable(gl.DEPTH_TEST);
  gl.enable(gl.CULL_FACE);

  const vao = gl.createVertexArray();
  gl.bindVertexArray(vao);

  const meshes = N.meshes.map((m) => {
    const buffer = gl.createBuffer();
    gl.bindBuffer(gl.ARRAY_BUFFER, buffer);
    gl.bufferData(gl.ARRAY_BUFFER, meshData(m.id), gl.STATIC_DRAW);
    return { buffer, vertices: m.vertices };
  });

  const attribute = ([name, size, offset]) => ({ loc: gl.getAttribLocation(program, name), size, offset });
  const vertexAttributes = N.vertex.attributes.map(attribute);
  const instanceAttributes = N.instance.attributes.map(attribute);
  for (const a of vertexAttributes.concat(instanceAttributes)) gl.enableVertexAttribArray(a.loc);
  for (const a of instanceAttributes) gl.vertexAttribDivisor(a.loc, 1);

  const uniform = (name) => gl.getUniformLocation(program, name);
  const sceneUniforms = N.uniforms.map(([name, size]) => ({ loc: uniform(name), size }));
  const uVP = uniform("uVP");
  const uEye = uniform("uEye");
  const uPass = uniform("uPass");
  const V = N.view;
  const instanceBuffers = [gl.createBuffer(), gl.createBuffer()];
  const P = N.passes;

  function enterPass(pass) {
    gl.uniform1f(uPass, pass);
    if (pass === P.glow) {
      gl.enable(gl.BLEND);
      gl.blendFunc(gl.ONE, gl.ONE);
      gl.depthMask(false);
      gl.cullFace(gl.BACK);
      return;
    }
    gl.disable(gl.BLEND);
    gl.depthMask(true);
    gl.cullFace(pass === P.ink ? gl.FRONT : gl.BACK);
  }

  let sceneDraws = new Uint32Array(0);

  function drawList(draws) {
    let current = -1;
    for (let i = 0; i < draws.length; i += N.draw_values) {
      const [buffer, meshId, byteOffset, count, pass] = draws.subarray(i, i + N.draw_values);
      if (pass !== current) {
        enterPass(pass);
        current = pass;
      }
      const mesh = meshes[meshId];
      if (!mesh.vertices) continue;
      gl.bindBuffer(gl.ARRAY_BUFFER, mesh.buffer);
      for (const a of vertexAttributes) {
        gl.vertexAttribPointer(a.loc, a.size, gl.FLOAT, false, N.vertex.stride, a.offset);
      }
      gl.bindBuffer(gl.ARRAY_BUFFER, instanceBuffers[buffer]);
      for (const a of instanceAttributes) {
        gl.vertexAttribPointer(a.loc, a.size, gl.FLOAT, false, N.instance.stride, a.offset + byteOffset);
      }
      gl.drawArraysInstanced(gl.TRIANGLES, 0, mesh.vertices, count);
    }
  }

  return {
    // The scenery never moves: uploaded once a fight, with its colours.
    setScene(values, draws, colours) {
      gl.bindBuffer(gl.ARRAY_BUFFER, instanceBuffers[N.buffers.static]);
      gl.bufferData(gl.ARRAY_BUFFER, values, gl.STATIC_DRAW);
      sceneDraws = draws;
      let at = 0;
      for (const u of sceneUniforms) {
        gl[`uniform${u.size}fv`](u.loc, colours.subarray(at, at + u.size));
        at += u.size;
      }
      const [r, g, b] = colours.subarray(at, at + N.clear);
      gl.clearColor(r, g, b, 1);
    },
    resize(width, height) {
      canvas.width = width;
      canvas.height = height;
      gl.viewport(0, 0, width, height);
    },
    frame(view, values, draws) {
      gl.bindBuffer(gl.ARRAY_BUFFER, instanceBuffers[N.buffers.dynamic]);
      gl.bufferData(gl.ARRAY_BUFFER, values, gl.STREAM_DRAW);
      gl.depthMask(true);
      gl.clear(gl.COLOR_BUFFER_BIT | gl.DEPTH_BUFFER_BIT);
      gl.uniformMatrix4fv(uVP, false, view.subarray(...V.matrix));
      gl.uniform3fv(uEye, view.subarray(...V.eye));
      // the scenery's solid and ink first, then the figures, then every glow
      drawList(sceneDraws);
      drawList(draws);
      enterPass(P.solid);
    },
  };
}
