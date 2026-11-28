"""Loar Fig. 3 through-bar. Two constrained translations and finite bearing rotation."""
import numpy as np
from scipy.linalg import eigh
from reed_element import beam_element


def modes(elements):
    supports=[round(elements*.36),round(elements*.67)]
    free=np.delete(np.arange(2*elements+2),[2*i for i in supports]); k=np.zeros((len(free)+2,)*2);m=np.zeros_like(k)
    el_k,el_m=beam_element(1/elements)
    for i in range(elements):
        ix=slice(2*i,2*i+4);k[ix,ix]+=el_k;m[ix,ix]+=el_m
    for i in supports:
        k[2*i+1,2*i+1]+=12 # Estimated rotational restraint in EI/L.
    ix=np.ix_(free,free)
    ev,p=eigh(k[ix],m[ix],subset_by_index=(0,9)); all_=np.zeros((len(free)+2,10));all_[free]=p
    print('fundamental coefficient',np.sqrt(ev[0]));
    for i in range(10):
        # Retain signed, mass-normalized participation at front and rear.
        shape=all_[::2,i];x=np.arange(elements+1)/elements
        print(f'({np.sqrt(ev[i]/ev[0]):.7f}, {np.interp(.008,x,shape):.7f}, {np.interp(.976,x,shape):.7f}, {np.interp(.994,x,shape):.7f}),')

if __name__=='__main__':modes(200)
