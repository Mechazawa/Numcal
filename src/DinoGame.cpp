#include "DinoGraphics.h"
#include "DinoGame.hpp"

unsigned char DinoGame::getAction(unsigned char row, unsigned char column)  const {
    return this->mapping[row][column];
}

void DinoGame::onPress(char row, char column) {
    const unsigned char action = this->getAction(row, column);
    // Deliberate choice for DINO_1 since floor_ will
    // be lower when the dino is crouched.
    const char floor_ = 30 - dinoSprites[EDinoSprites::DINO_1].height;

    switch(action) {
        case RESTART:
            this->reset();
            break;
        case SPAWN:
            this->spawnMob();
            break;
        case JUMP:
            if (this->dino.x != floor_) break;
            this->vx = -30;
            break;
        case CROUCH:
            if (this->dino.x != floor_) break;

            const unsigned char newFrames[] = {EDinoSprites::DINO_6, EDinoSprites::DINO_7};

            this->dino.animation.setFrames(newFrames, 2);
            this->dino.y = 30 - this->dino.animation.getSprite()->height;
            break;
    }
}

void DinoGame::onRelease(char row, char column) {
        const unsigned char action = this->getAction(row, column);

    switch(action) {
        case CROUCH:
            const unsigned char newFrames[] = {EDinoSprites::DINO_3, EDinoSprites::DINO_4};

            this->dino.animation.setFrames(newFrames, 2);
            this->dino.y = 30 - this->dino.animation.getSprite()->height;
            break;
    }
}

void DinoGame::draw(U8G2* u8g2) {
    u8g2->clearBuffer();
    u8g2->drawLine(0, SCREEN_HEIGHT - 1, SCREEN_WIDTH, SCREEN_HEIGHT - 1);
    
    // for(int i = 0; i < MAX_ENTITIES; i++) {
    //     if (this->entities[i].dead) continue;
    //     this->entities[i].draw(u8g2);
    // }

    this->dino.draw(u8g2);

    char scoreText[12];
    String(this->score).toCharArray(scoreText, sizeof(scoreText));

    u8g2->setFont(u8g2_font_baby_tn);
    u8g2->drawStr(0, 5, scoreText);

    u8g2->sendBuffer();
}

void DinoGame::tick(const unsigned long ms) {
    unsigned int delta = ms - this->lastTick;
    
    this->lastTick = ms;

    const Sprite* dinoSprite = this->dino.animation.getSprite();
    const short collisionEdge = this->dino.x + dinoSprite->width;

    if (this->vx < MAX_SPEED) {
        this->vx += min(((float)delta / 1000) * GRAVITY, MAX_SPEED);
    }

    for(int i = 0; i < MAX_ENTITIES; i++) {
        Entity* entity = &this->entities[i];

        if(entity->dead) {
            continue;
        }

        entity->tick(delta);

        if (this->dino.dead) {
            continue;
        }

        entity->x -= ((float)delta / 1000) * (float)this->speed;

        // collision todo move
        const Sprite* sprite = entity->animation.getSprite();

        if(entity->x + sprite->width < 0) {
            entity->dead = true;
            continue;
        }
        
        if (entity->collision && entity->x <= collisionEdge) {
            unsigned char newFrames[] = {EDinoSprites::DINO_5};

            this->dino.animation.setFrames(newFrames, 1);
            this->dino.kill();
        }
    }

    this->dino.tick(delta);

    if (!this->dino.dead) {
        this->score += ((float)delta / 1000) * 3;
    }
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
        this->entities[i].kill();
    }

    this->speed = 10;
    this->score = 0;
    this->vx = 0;

    // const unsigned char newFrames[] = {EDinoSprites::DINO_3, EDinoSprites::DINO_4};

    // this->dino = Entity(
    //     7, 30 - dinoSprites[newFrames[0]].height,
    //     Animation(newFrames, 2)
    // );

    this->dino.animation.clearFrames();
    this->dino.animation.addFrame(EDinoSprites::DINO_3);
    this->dino.animation.addFrame(EDinoSprites::DINO_4);
    this->dino.y = 30 - this->dino.animation.getSprite()->height;
}

void DinoGame::spawnMob() {
    const unsigned char id = this->findDeadEntity();
    const unsigned char newFrames[] = {EDinoSprites::BIRD_1, EDinoSprites::BIRD_2};

    this->entities[id] = Entity(128, 0, Animation(newFrames, 2, 400));

    Serial.println("SPAWN");
}
