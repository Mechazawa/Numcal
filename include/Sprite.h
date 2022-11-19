#pragma once

typedef struct Sprite {
    char width;
    char height;
    const unsigned char* data;
} Sprite;


/*
// For conversion, just input graphic as string 01110001110001
art2xbm = data => data.replace(/,? *0b/g, '')
                      .replace(/[^01\n]/g, '')
					  .split('\n')
					  .map(x => x.trim())
					  .map(x =>
					  	x.replace(/([01]{1,8})/g, '.$1')
					  	 .split('.')
					  	 .filter(x => x.length)
					  	 .map(x => x.split("").reverse().join(""))
					  )
					  .flat()
					  .map(x=>'0x' + ('00' + eval('0b'+x).toString(16)).slice(-2))
					  .join(', ')

xbm2art = (data, width) => data.map(x=>
                                    ('00000000' + x.toString(2)).slice(-8)
                                    .split("").reverse().join("")
                                )
                                .join('')
                                .replace(new RegExp('([01]{'+Math.ceil(width/8)*8+'})', 'g'), '$1\n')
*/