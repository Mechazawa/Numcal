#include "KeyboardConfig.h"
#include "KeyboardInterface.hpp"
#include "DinoGraphics.h"

#pragma once

#define MAX_ENTITIES 16
#define SCREEN_WIDTH 128
#define SCREEN_HEIGHT 32

typedef struct Entity {
    float x;
    short y;
    bool collision;
    unsigned char frame;
    unsigned char frames[16];
    unsigned char frame_count;
    unsigned short animation_delay;
    unsigned short animation_frametime;
    bool dead;
} Entity;

class DinoGame : public KeyboardInterface {
    private:
        Entity entities[MAX_ENTITIES];
        Entity dino;

        const unsigned char mapping[ROWS][COLS] = {
            {0, 0, 0, 0},
            {0, 0, 0, 0},
            {0, 0, 0, 0},
            {0, 0, 0, 0},
            {0, 0, 0, 0},
            {0, 0, 0, 0},
        };

        unsigned long lastTick = 0;
        float speed = 3;
        
    protected:
        unsigned char getAction(unsigned char row, unsigned char column) const;
        unsigned char findDeadEntity() const;
        void drawEntity(U8G2* u8g2, const Entity* entity);
        void tickAnimation(unsigned int delta, Entity* entity);

    public:
        void onPress(char row, char column) override;
        void draw(U8G2* u8g2) override;
        void tick(const unsigned long ms) override;
        void onShow() override;

        void reset();
};