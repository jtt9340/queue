![Queue Logo](Line Of People Clipart.jpg)
# Queue
A Slack bot that keeps track of people waiting in line for an event and notifies next in line when it is their turn

---
[Computer Science House](https://csh.rit.edu) (CSH) is an organization at the [Rochester Institute of Technology](https://www.rit.edu) (RIT) that has several 3D printers. Members of CSH currently use sticky notes to keep track of whose turn it is to use a particular 3D printer. This Slack app serves to automate the waiting process and ditch the sticky notes.

## Usage
As this app is still in development, this is subject to change. However, here is how you use Queue *for now*.

All Queue commands are invoked by mentioning Queue (`@Queue`) in a message you post in a channel where the Queue app is installed.
* __add__ - `@Queue add`
	* Add yourself to the Queue if you are not already in it.
* __cancel__ - `@Queue cancel`
	* If you are not at the front of the Queue, leave early. The person behind you, if there is one, is not notifed.
* __done__ - `@Queue done`
	* If you are at the front of the Queue, exit the Queue. The person behind you, if there is one, is notified.
* __show__ - `@Queue show`
	* See who is currently in the Queue and what position they are in.

## Sample Run

TODO

## ideas.txt
This repo contains a file titled `ideas.txt` containing ways that this app can be improved. For starters, the commands
__cancel__ and __done__ are redundant and can be combined into one. 
	
