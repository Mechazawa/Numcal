#pragma once

#include "Sprite.h"
#include "Animation.hpp"
#include <U8g2lib.h>

class Entity {
    public:
        Entity(short x = 0, short y = 0) ;
        Entity(short x, short y, Animation animation);
    
        float x;
        short y;
        bool collision;
        bool dead;
        Animation animation;

        void draw(U8G2* u8g2) const;
        void tick(const unsigned int delta);
        void kill();
};

