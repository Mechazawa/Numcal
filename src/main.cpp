#include <Arduino.h>
#include "HID-Project.h"
#include "KeyboardConfig.h"
#include "Numpad.hpp"
#include "Calculator.hpp"

#define DEBUG

Numpad numpad;
Calculator calculator;

KeyboardInterface* currentMode = &numpad;

const uint8_t colPins[COLS] = {A3, A2, A1, A0};
const uint8_t rowPins[ROWS] = {9, 8, 7, 6, 5, 4};

bool tick = false;

unsigned long states[COLS][ROWS];

unsigned short longPressMs = 1000;

void setup()
{
  NKROKeyboard.begin();
  Serial.begin(9600);

  for (uint8_t pin : colPins) {
    pinMode(pin, INPUT_PULLUP);
  }
  
  for (uint8_t pin : rowPins) {
    pinMode(pin, OUTPUT);
    digitalWrite(pin, HIGH);
  }

  Serial.println("Ready");
}

void loop()
{
  const unsigned long time = millis();

  for (uint8_t row = 0; row < ROWS; row++) {
    digitalWrite(rowPins[row], LOW);

    for (uint8_t col = 0; col < COLS; col++) {
      const unsigned long prev = states[col][row];
      const bool now = digitalRead(colPins[col]) == LOW; // todo maybe needs to be flipped?

      if (now != (prev > 0)) {
        #ifdef DEBUG
        Serial.print("["); Serial.print(row, DEC); Serial.print("]");
        Serial.print("["); Serial.print(col, DEC); Serial.print("]");
        Serial.println(now ? "DOWN" : "UP");
        #endif

        if (now) {
          states[col][row] = time;
          currentMode->onPress(row, col);
        } else {
          states[col][row] = 0;
          currentMode->onRelease(row, col);
        }
      } else if (now && (time - prev) >= longPressMs) {
        states[col][row] = time; // reset

        #ifdef DEBUG
        Serial.print("["); Serial.print(row, DEC); Serial.print("]");
        Serial.print("["); Serial.print(col, DEC); Serial.print("]");
        Serial.println("LONG");
        #endif

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

    digitalWrite(rowPins[row], HIGH);
  }
}