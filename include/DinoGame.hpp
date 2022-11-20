#include "KeyboardConfig.h"
#include "KeyboardInterface.hpp"
#include "Entity.hpp"

#pragma once

#define MAX_ENTITIES 12
#define SCREEN_WIDTH 128
#define SCREEN_HEIGHT 32
#define GRAVITY 8
#define MAX_SPEED 40

enum EInputAction : unsigned char {
    NONE, JUMP, CROUCH, RESTART, SPAWN
};

class DinoGame : public KeyboardInterface {
    private:
        Entity entities[MAX_ENTITIES];
        Entity dino;

        const unsigned char mapping[ROWS][COLS] = {
            {NONE,    NONE, NONE, NONE},
            {RESTART, SPAWN, NONE, NONE},
            {NONE,    NONE, NONE, JUMP},
            {NONE,    NONE, NONE, JUMP},
            {NONE,    NONE, NONE, CROUCH},
            {JUMP,    JUMP, CROUCH, CROUCH},
        };

        unsigned long lastTick = 0;
        unsigned short speed;
        int score;
        float vx;
        
    protected:
        unsigned char getAction(unsigned char row, unsigned char column) const;
        unsigned char findDeadEntity() const;
        void drawEntity(U8G2* u8g2, const Entity* entity);

    public:
        void onPress(char row, char column) override;
        void onRelease(char row, char column) override;
        void draw(U8G2* u8g2) override;
        void tick(const unsigned long ms) override;
        void onShow() override;
        void spawnMob();

        void reset();
};