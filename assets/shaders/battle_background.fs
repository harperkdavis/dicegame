
#version 330

in vec2 fragTexCoord;
in vec4 fragColor;

uniform sampler2D tex0;
uniform float time;

out vec4 finalColor;

const float SPEED = 0.04;

// 4x4 Bayer threshold
float bayer4x4(vec2 p) {
    ivec2 ip = ivec2(mod(p, 4.0));

    int index =
        ip.x == 0 && ip.y == 0 ? 0  :
        ip.x == 1 && ip.y == 0 ? 8  :
        ip.x == 2 && ip.y == 0 ? 2  :
        ip.x == 3 && ip.y == 0 ? 10 :
        ip.x == 0 && ip.y == 1 ? 12 :
        ip.x == 1 && ip.y == 1 ? 4  :
        ip.x == 2 && ip.y == 1 ? 14 :
        ip.x == 3 && ip.y == 1 ? 6  :
        ip.x == 0 && ip.y == 2 ? 3  :
        ip.x == 1 && ip.y == 2 ? 11 :
        ip.x == 2 && ip.y == 2 ? 1  :
        ip.x == 3 && ip.y == 2 ? 9  :
        ip.x == 0 && ip.y == 3 ? 15 :
        ip.x == 1 && ip.y == 3 ? 7  :
        ip.x == 2 && ip.y == 3 ? 13 :
                                  5;

    return float(index) / 16.0;
}

const vec3 DARK_COLOR = vec3(22.0 / 255.0, 13.0 / 255.0, 69.0 / 255.0);
const vec3 LIGHT_COLOR = vec3(143.0 / 255.0, 123.0 / 255.0, 179.0 / 255.0);

void main() {
    vec4 upLeftTexelFast =
        texture(tex0, fragTexCoord + vec2(0.33, 0.0) + time * SPEED)
        * (abs(sin(time)) * 0.5 + 0.5);

    vec4 downRightTexelFast =
        texture(tex0, fragTexCoord - time * SPEED)
        * (abs(cos(time)) * 0.5 + 0.5);

    vec4 upLeftTexelSlow =
        texture(tex0, fragTexCoord + vec2(0.66, 0.5) + time * SPEED * 0.5)
        * (abs(cos(time * 2.0)) * 0.5 + 0.5);

    vec4 downRightTexelSlow =
        texture(tex0, fragTexCoord + vec2(0.33, 0.5) - time * SPEED * 0.5)
        * (abs(sin(time * 2.0)) * 0.5 + 0.5);
    
    vec3 gradientToTopLeft = vec3(fragTexCoord.x - fragTexCoord.y);

    vec3 color = (upLeftTexelFast.rgb * 0.5 * LIGHT_COLOR + downRightTexelFast.rgb * 0.5 * LIGHT_COLOR + upLeftTexelSlow.rgb * DARK_COLOR * 2.0 + downRightTexelSlow.rgb * DARK_COLOR * 2.0 + gradientToTopLeft) / 6.0;

    float threshold = bayer4x4(gl_FragCoord.xy / 2.0);

    // Simulate low bit depth (e.g. 5 bits per channel)
    float levels = 8.0;
    color = floor(color * levels + threshold) / levels;

    finalColor = vec4(color, 1.0);
}
