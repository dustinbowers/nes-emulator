const vec2 size = vec2(320, 240);
const float pi = 3.142;


void mainImage( out vec4 f, vec2 p) {

   p *= .5/iResolution.xy;
   vec2 pos = floor(p*size);
   
   	vec4 res = texture2D(iChannel0, pos/size);
    vec3 yuv = res.xyz * mat3( 0.2126,   0.7152,   0.0722,
                              -0.09991, -0.33609,  0.436,
                               0.615,   -0.55861, -0.05639);

    vec3 rgb = yuv*mat3( 1,  0,         1.28033,
                         1, -0.21482 , -0.38059,
                         1,  2.12798,   0 );

    vec2 ap = floor(p*size*vec2(4,.5));
    float a = ( dot(ap,vec2(.5,2./3.)) +.25)*pi;

    vec2 sincosv = .5-.5*sin(a + vec2(0,pi/2.));
    if (mod(pos.y,2.) < 1.)   sincosv.x = 1.-sincosv.x;
    
    float mc = dot(sincosv,yuv.yz+.5);
    
    f =  vec4(rgb*mc,1);
}