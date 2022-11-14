#include <Arduino.h>
#include "HID-Project.h"
#include "KeyboardConfig.h"
#include "Numpad.hpp"
#include "Calculator.hpp"

Numpad numpad;
Calculator calculator;

KeyboardInterface* currentMode = &numpad;

const uint8_t colPins[] = {A0, A1, A2, A3};
const uint8_t rowPins[] = {9, 8, 7, 6, 5, 4};

bool tick = false;

unsigned long states[COLS][ROWS];

unsigned short longPressMs = 2000;

void setup()
{
  Keyboard.begin(); 

  for (uint8_t pin : rowPins) {
    pinMode(pin, INPUT_PULLUP);
  }
  
  for (uint8_t pin : colPins) {
    pinMode(pin, OUTPUT);
    digitalWrite(pin, HIGH);
  }
}

void loop()
{
  const unsigned long time = millis();

  for (uint8_t col = 0; col < COLS; col++) {
    digitalWrite(colPins[col], LOW);

    for (uint8_t row = 0; row < ROWS; row++) {
      const unsigned long prev = states[col][row];
      const bool now = digitalRead(rowPins[row]) == LOW; // todo maybe needs to be flipped?

      if (now != (prev > 0)) {
        if (now) {
          states[col][row] = time;
          currentMode->onPress(row, col);
        } else {
          states[col][row] = 0;
          currentMode->onRelease(row, col);
        }
      } else if (now && (time - prev) >= longPressMs) {
        states[col][row] = time; // reset

        // bad way of doing this
        // detect numlock and swich modes
        if (col == 0 && row == 1) {
          if (currentMode == &numpad) {
            currentMode = &calculator;
          } else {
            currentMode = &numpad;
          }
        } else {
          currentMode->onLongPress(col, row);
        }
      }
    }

    digitalWrite(colPins[col], HIGH);
  }
}