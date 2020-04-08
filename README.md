# Queue
A Slack bot that keeps track of people waiting in line for an event and notifies next in line when it is their turn

---
[Computer Science House](https://csh.rit.edu) (CSH) is an organization at the [Rochester Institute of Technology](https://www.rit.edu) (RIT) that has several 3D printers. Members of CSH currently use sticky notes to keep track of whose turn it is to use a particular 3D printer. This Slack app serves to automate the waiting process and ditch the sticky notes.

## Usage
As this app is still in development, this is subject to change. However, here is how you use Queue *for now*.

All Queue commands are invoked by mentioning Queue (`@Queue`) in a message you post in a channel where the Queue app is installed. You immediately follow the `@Queue` mention with __one__ of the following words (commands do not yet take arguments, although it doesn't make sense for any of the following commands to currently take arguments. Queue may accept certain commands in the future that take required and/or optional arguments):
* __add__ - `@Queue add`
	* Add yourself to the Queue. New users are quick to notice that they can add themselves to the queue multiple times.
	That is not a bug—it's a feature! Suppose you have multiple things you want to 3D print. You add yourself to the queue
	*once* for *each* thing you want to 3D print. However, there are some rules to curb people trying hog the entire queue
	by, say, adding ten back-to-back instances of themselves to the queue (not that anyone would do that 😉). The rules are:
		1. You can only add yourself to the queue if the last person in line is __not__ yourself (i.e. you cannot have two
		back-to-back instances of yourself), unless...
		2. ...if the queue is completely empty when you join, you can have up to three back-to-back instances of yourself.
* __done__ - `@Queue done`
	* The instance of yourself *closest to the front of the line* leaves the queue. If that instance was first in line, then
	the person who *was* in second place (now in first) is notified of their new position!
* __show__ - `@Queue show`
	* See who is currently in the Queue and what position they are in.
* __help__ - `@Queue help`
	* Display a man-page style help message in case you forget what commands you can do.

## Sample Run

[Queue Demo](./Queue Demo Short.gif)

## ideas.txt
This repo contains a file titled `ideas.txt` containing ways that this app can be improved. For instance, there is
(currently) no time limit as to how long one can stay at the front of the queue. This is planned to change.
