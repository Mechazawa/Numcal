#pragma once
#include "Sprite.h"
#include "DinoGraphics.h"

#define MAX_ANIMATION_FRAMES 8

class Animation {
    private:
        unsigned char frame = 0;
        unsigned char frames[MAX_ANIMATION_FRAMES];
        unsigned char frameCount = 0;
        unsigned short frameTime;

    public:
        unsigned short animationDelay;
        const Sprite* spriteSet;

        Animation(const unsigned char* frames = {}, const unsigned char size = 0, unsigned short animationDelay = 200, const Sprite* spriteSet = dinoSprites);

        void setFrames(const unsigned char* frames, const unsigned char size);
        bool addFrame(unsigned char frame);
        void clearFrames();
        void nextFrame();
        const Sprite* getSprite() const;
        void tick(const unsigned int delta);
};