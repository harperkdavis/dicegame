
#version 330

in vec2 fragTexCoord;
in vec4 fragColor;

uniform sampler2D tex0;
uniform float main_time;
uniform float end_time;

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
    float anim_in = exp(-main_time / 1.5);
    float anim_out = 0.0;
    vec2 movement = vec2(main_time - anim_in * 20.0, main_time) * SPEED;

    float stretch = 1.0;
    vec3 dark_color = DARK_COLOR;
    vec3 light_color = LIGHT_COLOR;
    vec3 gradientToTopLeft = vec3(fragTexCoord.x - fragTexCoord.y);
    if (end_time < 0.0) {
        stretch = 1.0 + pow(end_time, 3.0) / 2.0;
        movement.x += pow(end_time, 5.0) * 0.2;

        anim_out = pow(min(abs(end_time), 1.0), 9.0);
    }
    if (end_time > 0.0) {
        stretch = exp(-end_time) * 2.0 + 4.0;
        movement.y += end_time * 8.0 * SPEED;

        dark_color = vec3(34.0 / 255.0, 4.0 / 255.0, 6.0 / 255.0);
        light_color = vec3(123.0 / 255.0, 143.0 / 255.0, 179.0 / 255.0);
        gradientToTopLeft = vec3(-fragTexCoord.y * 2.0 + 0.5);

        anim_out = exp(-end_time * 8.0);
    }
    vec2 sample = (fragTexCoord - vec2(0.25, 0.0)) * vec2(stretch, 1.0);

    vec4 upLeftTexelFast =
        texture(tex0, sample + vec2(0.33, 0.0) + movement)
        * (abs(sin(main_time)) * 0.5 + 0.5);

    vec4 downRightTexelFast =
        texture(tex0, sample - movement)
        * (abs(cos(main_time)) * 0.5 + 0.5);

    vec4 upLeftTexelSlow =
        texture(tex0, sample + vec2(0.66, 0.5) + movement * 0.5) 
        * (abs(cos(main_time * 2.0)) * 0.5 + 0.5);

    vec4 downRightTexelSlow =
        texture(tex0, sample + vec2(0.33, 0.5) - movement * 0.5)
        * (abs(sin(main_time * 2.0)) * 0.5 + 0.5);
    

    vec3 color = (upLeftTexelFast.rgb * 0.5 * light_color + downRightTexelFast.rgb * 0.5 * light_color + upLeftTexelSlow.rgb * dark_color * 2.0 + downRightTexelSlow.rgb * dark_color * 2.0 + gradientToTopLeft) / 6.0 * stretch * (anim_in + 1.0) + anim_in * anim_in + anim_out;

    float threshold = bayer4x4(gl_FragCoord.xy / 2.0);

    // Simulate low bit depth (e.g. 5 bits per channel)
    float levels = 8.0;
    color = floor(color * levels + threshold) / levels;

    finalColor = vec4(color, 1.0);
}
