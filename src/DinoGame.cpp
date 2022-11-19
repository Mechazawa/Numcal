#include "DinoGame.hpp"
#include "DinoGraphics.h"

unsigned char DinoGame::getAction(unsigned char row, unsigned char column)  const {
    return this->mapping[row][column];
}

void DinoGame::onPress(char row, char column) {
    const unsigned char action = this->getAction(row, column);

    switch(action) {
        case 0:
            this->entities[this->findDeadEntity()] = {
                128, 8, true, 0, 
                {EDinoSprites::CACTUS_1}, 
                1, 1000, 0, false
            };
            break;
        case 1:
            this->entities[this->findDeadEntity()] = {
                128, 5, true, 0, 
                {EDinoSprites::DINO_6, EDinoSprites::DINO_7}, 
                2, 200, 0, false
            };
            break;
        case 2:
            this->entities[this->findDeadEntity()] = {
                128, 0, true, 0, 
                {EDinoSprites::BIRD_1, EDinoSprites::BIRD_2}, 
                2, 400, 0, false
            };
            break;
        case 3:
            this->entities[this->findDeadEntity()] = {
                128, 8, true, 0, 
                {EDinoSprites::CACTUS_2}, 
                1, 1000, 0, false
            };
            break;
        case 4:
            this->entities[this->findDeadEntity()] = {
                128, 8, true, 0, 
                {EDinoSprites::CACTUS_3}, 
                1, 1000, 0, false
            };
            break;
    }
}

void DinoGame::draw(U8G2* u8g2) {
    u8g2->clearBuffer();
    u8g2->drawLine(0, SCREEN_HEIGHT - 1, SCREEN_WIDTH, SCREEN_HEIGHT - 1);
    
    for(int i = 0; i < MAX_ENTITIES; i++) {
        const Entity* entity = &this->entities[i];
        
        if(!entity->dead) {
            this->drawEntity(u8g2, entity);
        }
    }

    this->drawEntity(u8g2, &this->dino);

    u8g2->sendBuffer();
}

void DinoGame::drawEntity(U8G2* u8g2, const Entity* entity) {
    const Sprite* sprite = &dinoSprites[entity->frames[entity->frame]];

    u8g2->drawXBMP(entity->x, entity->y, sprite->width, sprite->height, sprite->data);
}

void DinoGame::tick(const unsigned long ms) {
    unsigned int delta = ms - this->lastTick;
    
    this->lastTick = ms;

    for(int i = 0; i < MAX_ENTITIES; i++) {
        Entity* entity = &this->entities[i];

        if(entity->dead) {
            continue;
        }

        entity->x -= ((float)delta / 1000) * this->speed;
        const Sprite* sprite = &dinoSprites[entity->frames[entity->frame]];

        if(entity->x + sprite->width < 0) {
            entity->dead = true;
        } else {
            this->tickAnimation(delta, entity);
        }
    }

    this->tickAnimation(delta, &this->dino);
}

void DinoGame::onShow() {
    this->reset();
}

unsigned char DinoGame::findDeadEntity() const {
    for(int i = 0; i < MAX_ENTITIES; i++) {
        if(this->entities[i].dead) {
            return i;
        }
    }

    return 0;
}

void DinoGame::reset() {
    for (int i = 0; i < MAX_ENTITIES; i++) {
        this->entities[i].dead = true;
    }

    this->dino = {
        7, 30 - dinoSprites[0].height, true, 0, 
        {EDinoSprites::DINO_3, EDinoSprites::DINO_4}, 
        2, 200, 0, false
    };
}

void DinoGame::tickAnimation(unsigned int delta, Entity* entity) {
    entity->animation_frametime += delta;

    if (entity->animation_frametime < entity->animation_delay) {
        return;
    }

    entity->frame++;

    if (entity->frame >= entity->frame_count) {
        entity->frame = 0;
    }

    entity->animation_frametime = 0;
}
