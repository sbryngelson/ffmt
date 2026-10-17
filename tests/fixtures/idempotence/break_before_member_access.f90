program t
if (a) then
if (b) then
if (c) then
if (d) then
print *, "Image point is located approximately ", &
    & (ghost_points(q)%loc(dim) - ghost_points(q) %ip_loc(dim))/(s_cc(1) - s_cc(0)), " grid cells away"
end if
end if
end if
end if
end program t
