#include "Entity.hpp"

Entity::Entity(short x, short y) {
    this->x = x;
    this->y = y;
}

Entity::Entity(short x, short y, Animation animation) {
    this->x = x;
    this->y = y;
    this->animation = animation;
}

void Entity::draw(U8G2* u8g2) const {
    if (!this->dead) {
        const Sprite* sprite = this->animation.getSprite();

        u8g2->drawXBMP(this->x, this->y, sprite->width, sprite->height, sprite->data);
    }
}

void Entity::tick(const unsigned int delta) {
    this->animation.tick(delta);
}

void Entity::kill() {
    this->dead = true;
}