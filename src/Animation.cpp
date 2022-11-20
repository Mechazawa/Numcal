#include "Animation.hpp"
#include "Arduino.h"

Animation::Animation(const unsigned char* frames, const unsigned char size, unsigned short animationDelay, const Sprite* spriteSet) {
    this->animationDelay = animationDelay;
    this->spriteSet = spriteSet;

    this->setFrames(frames, size);
}

bool Animation::addFrame(unsigned char frame) {
    if (this->frameCount >= MAX_ANIMATION_FRAMES) {
        return false;
    }

    this->frames[this->frameCount] = frame;
    this->frameCount++;

    return true;
}

void Animation::clearFrames() {
    this->frameCount = 0;
    this->frame = 0;
}

void Animation::nextFrame() {
    this->frame = (this->frame + 1) % this->frameCount;
    this->frameTime = 0;
}

const Sprite* Animation::getSprite() const {
    if (this->frameCount == 0) {
        return &this->spriteSet[0];
    }

    const unsigned char id = this->frames[this->frame];

    return &this->spriteSet[id];
}

void Animation::tick(const unsigned int delta) {
    this->frameTime += delta;

    if (this->frameTime >= this->animationDelay) {
        this->nextFrame();
    }
}

void Animation::setFrames(const unsigned char* frames, const unsigned char size) {
    this->clearFrames();

    this->frameCount = min(MAX_ANIMATION_FRAMES, size);

    for (int i = 0; i < this->frameCount; i++) {
        this->frames[i] = frames[i];
    }
}