#include <stdio.h>
#include <stdlib.h>
#include <sys/times.h>
#define W 79
#define H 24
/* 79x24 like MANDEL.COM: 8.8 fixed point, 15 iterations */
char shade[]="#@@%%**++==--::. ";
int img[H][W];
int fixed(int x,int y)
{
 short zr=0,zi=0,r2,i2;int n=15;
 while(n){r2=(short)(zr*zr>>8);i2=(short)(zi*zi>>8);
  if((short)(r2+i2)>1024)break;
  zi=(short)((short)(zr*zi>>8)*2+y);zr=(short)(r2-i2+x);n--;}
 return n;
}
int flt(int x,int y)
{
 double cr=x/256.0,ci=y/256.0,zr=0,zi=0,r2,i2;int n=15;
 while(n){r2=zr*zr;i2=zi*zi;if(r2+i2>4.0)break;
  zi=2*zr*zi+ci;zr=r2-i2+cr;n--;}
 return n;
}
int main(int argc,char **argv)
{
 int f=argc>1&&argv[1][0]=='f',reps=argc>2?atoi(argv[2]):1;
 int r,i,j,x,y,n;long sum=0;struct tms t0,t1;clock_t c0,c1;
 c0=times(&t0);
 for(r=0;r<reps;r++)for(i=0,y=-288;i<H;i++,y+=25)
  for(j=0,x=-512;j<W;j++,x+=10)img[i][j]=f?flt(x,y):fixed(x,y);
 c1=times(&t1);
 for(i=0;i<H;i++){for(j=0;j<W;j++){n=img[i][j];sum+=n;
  printf("\033[%d;3%dm%c",n>7,n?n%6+1:7,shade[n]);}
  printf("\033[0m\n");}
 printf("%s x%d: %ld ticks user, %ld real, sum %ld\n",f?"float":"fixed",
  reps,(long)(t1.tms_utime-t0.tms_utime),(long)(c1-c0),sum);
 return 0;
}
