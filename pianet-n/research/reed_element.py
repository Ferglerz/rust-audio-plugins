"""Hermite beam-element bending stiffness and consistent mass for unit EI and ρA."""
import numpy as np


def beam_element(h):
    stiffness = np.array([[12, 6*h, -12, 6*h], [6*h, 4*h*h, -6*h, 2*h*h],
                          [-12,-6*h,12,-6*h], [6*h,2*h*h,-6*h,4*h*h]]) / h**3
    mass = np.array([[156,22*h,54,-13*h], [22*h,4*h*h,13*h,-3*h*h],
                     [54,13*h,156,-22*h], [-13*h,-3*h*h,-22*h,4*h*h]]) * h/420
    return stiffness,mass
