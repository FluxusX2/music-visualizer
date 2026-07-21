import { useEffect, useRef } from "react";
import { listen } from "@tauri-apps/api/event";
import type { SongInfo } from "./PlayList.tsx";

const RPM = 25;
const ROTATION_SPEED = (2 * Math.PI * (RPM)) / 60;

let globalVinylAngle = 0;

const vertexShaderSrc = `#version 300 es
in vec2 a_position;
void main() {
    gl_Position = vec4(a_position, 0.0, 1.0);
}`;

const fragmentShaderSrc = `#version 300 es
precision highp float;
uniform vec2 u_resolution;
uniform float u_angle;
uniform float u_hasCover;
uniform sampler2D u_cover;
out vec4 outColor;

mat2 rotate(float a) {
    float c = cos(a);
    float s = sin(a);
    return mat2(c, -s, s, c);
}

void main() {
    vec2 uv = (gl_FragCoord.xy * 2.0 - u_resolution) / min(u_resolution.x, u_resolution.y);
    float r = length(uv);
    float theta = atan(uv.y, uv.x);
    
    // Automatically calculate the width of a single pixel for perfect smoothing
    float fw = fwidth(r); 

    const float labelRadius = 0.4;
    const float spindleRadius = 0.035;

    vec3 vinylDark = vec3(0.03, 0.03, 0.035);
    vec3 vinylLight = vec3(0.09, 0.09, 0.1);

    // Grooves
    float rawGroove = 0.5 + 0.5 * sin(r * 400.0);
    float groove = pow(rawGroove, 4.0);
    
    vec3 color = mix(vinylDark, vinylLight, groove * 0.8);
    // Sheen
    float sheen = pow(max(0.0, sin(theta * 2.0 - 1.0)), 8.0) * 0.25;
    color += sheen;

    // Label area
    if (r < labelRadius) {
        vec2 labelUv = rotate(-u_angle) * uv;
        vec2 texUv = (labelUv / labelRadius) * 0.5 + 0.5;
        
        // REMOVED: texUv.y = 1.0 - texUv.y;

        vec3 labelColor;
        if (u_hasCover > 0.5) {
            labelColor = texture(u_cover, texUv).rgb;
        } else {
            labelColor = vec3(0.55, 0.55, 0.58);
        }

        // Crisp, 1-pixel anti-aliased edge for the label
        float labelEdge = smoothstep(labelRadius - fw, labelRadius, r);
        color = mix(labelColor, color, labelEdge);
        
        // FIXED: Subtle dark ring ONLY on the outer edge of the label
        float ring = smoothstep(labelRadius - 0.015, labelRadius, r);
        
        // Darkens the very edge slightly, leaving the center at full brightness
        color = mix(color, vec3(0.0), ring * 0.4); 
    }

    // Spindle hole
    if (r < spindleRadius) {
        // Crisp edge for the center hole
        float spindleEdge = smoothstep(spindleRadius - fw, spindleRadius, r);
        color = mix(vinylDark * 0.2, color, spindleEdge);
    }

    // Outer edge of the vinyl
    // Uses fw * 1.5 for a tiny bit of extra smoothness on the absolute outer edge
    float alpha = 1.0 - smoothstep(1.0 - (fw * 1.5), 1.0, r); 
    
    outColor = vec4(color, alpha);
}`;

function compileShader(gl: WebGL2RenderingContext, type: number, source: string) {
    const shader = gl.createShader(type)!;
    gl.shaderSource(shader, source);
    gl.compileShader(shader);
    if (!gl.getShaderParameter(shader, gl.COMPILE_STATUS)) {
        console.error(gl.getShaderInfoLog(shader));
        gl.deleteShader(shader);
        return null;
    }
    return shader;
}

export default function Content({ song }: { song?: SongInfo | null }) {
    const canvasRef = useRef<HTMLCanvasElement>(null);
    const glRef = useRef<WebGL2RenderingContext | null>(null);
    const textureRef = useRef<WebGLTexture | null>(null);
    const hasCoverRef = useRef(false);
    const isPausedRef = useRef(true);

    // Set up the WebGL context, shader program and render loop once.
    useEffect(() => {
        const canvas = canvasRef.current;
        if (!canvas) return;

        const gl = canvas.getContext("webgl2", { alpha: true, premultipliedAlpha: false });
        if (!gl) {
            console.error("WebGL2 not supported");
            return;
        }
        glRef.current = gl;

        const vs = compileShader(gl, gl.VERTEX_SHADER, vertexShaderSrc);
        const fs = compileShader(gl, gl.FRAGMENT_SHADER, fragmentShaderSrc);
        if (!vs || !fs) return;

        const program = gl.createProgram()!;
        gl.attachShader(program, vs);
        gl.attachShader(program, fs);
        gl.linkProgram(program);
        if (!gl.getProgramParameter(program, gl.LINK_STATUS)) {
            console.error(gl.getProgramInfoLog(program));
            return;
        }

        const positionLoc = gl.getAttribLocation(program, "a_position");
        const resolutionLoc = gl.getUniformLocation(program, "u_resolution");
        const angleLoc = gl.getUniformLocation(program, "u_angle");
        const hasCoverLoc = gl.getUniformLocation(program, "u_hasCover");
        const coverLoc = gl.getUniformLocation(program, "u_cover");

        // Fullscreen triangle covering the clip space.
        const positions = new Float32Array([-1, -1, 3, -1, -1, 3]);
        const vao = gl.createVertexArray();
        gl.bindVertexArray(vao);
        const buffer = gl.createBuffer();
        gl.bindBuffer(gl.ARRAY_BUFFER, buffer);
        gl.bufferData(gl.ARRAY_BUFFER, positions, gl.STATIC_DRAW);
        gl.enableVertexAttribArray(positionLoc);
        gl.vertexAttribPointer(positionLoc, 2, gl.FLOAT, false, 0, 0);

        // Placeholder 1x1 texture until the cover art (if any) finishes loading.
        const texture = gl.createTexture();
        gl.bindTexture(gl.TEXTURE_2D, texture);
        gl.texImage2D(gl.TEXTURE_2D, 0, gl.RGBA, 1, 1, 0, gl.RGBA, gl.UNSIGNED_BYTE, new Uint8Array([140, 140, 148, 255]));
        gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_S, gl.CLAMP_TO_EDGE);
        gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_T, gl.CLAMP_TO_EDGE);
        gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MIN_FILTER, gl.LINEAR);
        textureRef.current = texture;

        gl.enable(gl.BLEND);
        gl.blendFunc(gl.SRC_ALPHA, gl.ONE_MINUS_SRC_ALPHA);

        const resizeObserver = new ResizeObserver(() => {
            const { clientWidth, clientHeight } = canvas;

            const RESOLUTION_MULTIPLIER = 2.0;

            const dpr = (window.devicePixelRatio || 1) * RESOLUTION_MULTIPLIER;

            canvas.width = Math.max(1, Math.round(clientWidth * dpr));
            canvas.height = Math.max(1, Math.round(clientHeight * dpr));
            gl.viewport(0, 0, canvas.width, canvas.height);
        });
        resizeObserver.observe(canvas);

        let animationFrame: number;
        let lastTime = performance.now();

        const render = (now: number) => {
            let dt = (now - lastTime) / 1000;
            lastTime = now;

            if (dt > 0.1) dt = 0.016;

            if (!isPausedRef.current) {
                globalVinylAngle = (globalVinylAngle + dt * ROTATION_SPEED) % (2 * Math.PI);
            }

            gl.clearColor(0, 0, 0, 0);
            gl.clear(gl.COLOR_BUFFER_BIT);

            gl.useProgram(program);
            gl.bindVertexArray(vao);
            gl.uniform2f(resolutionLoc, canvas.width, canvas.height);
            gl.uniform1f(angleLoc, globalVinylAngle);
            gl.uniform1f(hasCoverLoc, hasCoverRef.current ? 1.0 : 0.0);

            gl.activeTexture(gl.TEXTURE0);
            gl.bindTexture(gl.TEXTURE_2D, textureRef.current);
            gl.uniform1i(coverLoc, 0);

            gl.drawArrays(gl.TRIANGLES, 0, 3);
            animationFrame = requestAnimationFrame(render);
        };
        animationFrame = requestAnimationFrame(render);

        return () => {
            cancelAnimationFrame(animationFrame);
            resizeObserver.disconnect();
            gl.deleteProgram(program);
            gl.deleteShader(vs);
            gl.deleteShader(fs);
            gl.deleteBuffer(buffer);
            gl.deleteVertexArray(vao);
            gl.deleteTexture(texture);
            glRef.current = null;
            textureRef.current = null;
        };
    }, []);

    // Track play/pause state from the backend so the vinyl only spins while playing.
    useEffect(() => {
        let unlisten: (() => void) | undefined;
        listen<boolean>("playback-state", (event) => {
            isPausedRef.current = event.payload;
        }).then((fn) => {
            unlisten = fn;
        });
        return () => {
            if (unlisten) unlisten();
        };
    }, []);

    // Upload the current song's cover art into the label texture whenever it changes.
    useEffect(() => {
        const gl = glRef.current;
        const texture = textureRef.current;
        if (!gl || !texture) return;

        if (!song?.cover_art || song.cover_art.length === 0) {
            hasCoverRef.current = false;
            return;
        }

        const byteArray = new Uint8Array(song.cover_art);
        const blob = new Blob([byteArray]);
        const url = URL.createObjectURL(blob);
        const image = new Image();
        let cancelled = false;

        image.onload = () => {
            if (cancelled) return;
            gl.bindTexture(gl.TEXTURE_2D, texture);
            gl.pixelStorei(gl.UNPACK_FLIP_Y_WEBGL, true);
            gl.texImage2D(gl.TEXTURE_2D, 0, gl.RGBA, gl.RGBA, gl.UNSIGNED_BYTE, image);
            gl.generateMipmap(gl.TEXTURE_2D);
            gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MIN_FILTER, gl.LINEAR_MIPMAP_LINEAR);
            gl.pixelStorei(gl.UNPACK_FLIP_Y_WEBGL, false);
            hasCoverRef.current = true;
        };
        image.onerror = () => {
            hasCoverRef.current = false;
        };
        image.src = url;

        return () => {
            cancelled = true;
            URL.revokeObjectURL(url);
        };
    }, [song]);

    return <canvas ref={canvasRef} className="content-canvas" />;
}