"""Euler–Bernoulli finite elements for the production film's shouldered reed.

Relative dimensions and the two contact locations are estimates. The bolted
part is fixed and the flexible shoulder occupies the first third of the span.
All vectors are normalized to unit displacement at the free end.
"""
import numpy as np
from scipy.linalg import eigh


from reed_element import beam_element


def section(elements):
    length = 1 / elements
    k = np.zeros((2 * elements + 2,) * 2)
    m = np.zeros_like(k)
    for i in range(elements):
        x = (i + .5) * length
        width = 1 + 2.8 * max(0, 1-x/.33)**2
        h = length
        stiffness,mass = beam_element(h)
        s = slice(2*i,2*i+4)
        k[s,s] += width*stiffness
        m[s,s] += width*mass
    eigen, v = eigh(k[2:,2:], m[2:,2:],subset_by_index=(0,7))
    v /= v[-2,:]
    positions = np.vstack([np.zeros(8),v[::2,:]])
    data=[]
    for i in range(8):
        phi = lambda x: np.interp(x,np.arange(elements+1)*length,positions[:,i])
        data.append((np.sqrt(eigen[i]/eigen[0]), v[:,i] @ m[2:,2:] @ v[:,i], phi(.68),phi(.84)))
    return np.array(data)

if __name__=='__main__':
    print('ratio, modal mass / (tip width × thickness × length × density), hammer, damper')
    a=section(128);b=section(256)
    assert np.max(abs(a-b)) < .02
    for row in b:
        print('    (' + ', '.join(f'{x:.7f}' for x in row) + '),')
